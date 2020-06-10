#![feature(decl_macro)]
#![feature(proc_macro_hygiene)]

use tokio::runtime::Runtime;
use maud::{html, Markup};
use futures::future::{Future, TryFuture, FutureExt};
use rocket::http::RawStr;
use rocket::{
    get, post,
    request::{self, FromRequest},
    routes, Request,
};

use rusqlite::params;

// code: 128b
fn b128(code: &str) -> String {
    "B".to_owned() + code
}

fn mark_job(job: &Job) -> String {
    mark(
        &job.job,
        &job.part,
        &job.qty,
        &job.start,
        &job.stop,
        &job.checksum,
        &job.chip,
        &job.vendor,
        &job.part_type,
        &job.date,
        &job.lot,
        &job.user,
    )
}

fn mark(
    order: &str,
    ic: &str,
    qty: &u64,
    start: &str,
    stop: &str,
    checksum: &str,
    rom: &str,
    vendor: &str,
    part_type: &str,
    product_date: &str,
    batch: &str,
    user: &str,
) -> String {
    let qty = &qty.to_string();
    format!(
        "\x02L\r\n\
        yUGB\r\n\
        1911uC001800040P008P008B9A4B5A5BAC50000\r\n\
        191100201800075: {}\r\n\
        1e0202001600040{}\r\n\
        1911uC001400040P008P008C1CFBAC50000\r\n\
        191100301400065: {}\r\n\
        1e0202001200040{}\r\n\
        1911uC001400200P008P008CAFDC1BF0000\r\n\
        191100201400225: {}\r\n\
        1e0202001200200{}\r\n\
        1911uC001000040P008P008BFAACABCCAB1BCE40000\r\n\
        191100201000085: {}\r\n\
        1911uC001000200P008P008BDE1CAF8CAB1BCE40000\r\n\
        191100201000245: {}\r\n\
        191100200800040CHECKSUM: {}\r\n\
        1911uC000800200P008P008B3CCCABDB1E0C2EB0000\r\n\
        191100200800245: {}\r\n\
        1911uC000600040P008P008B3A7BCD20000\r\n\
        191100200600065: {}\r\n\
        1e0202000400040{}\r\n\
        191100200600200TYPE: {}\r\n\
        1911uC000200040P008P008D6C6D4ECC8D5C6DA0000\r\n\
        191100200200090: {}\r\n\
        1e0202000000040{}\r\n\
        1911uC000200200P008P008B3F6B3A7C5FABAC50000\r\n\
        191100200200250: {}\r\n\
        1e0202000000200{}\r\n\
        1W1d4400001300300P{};L{};D{};V{};Q{};R{};U{}\r\n\
        \r\n\
        E\r\n\
        ",
        order,
        b128(order),
        ic,
        b128(ic),
        qty,
        b128(qty),
        start,
        stop,
        checksum,
        rom,
        vendor,
        b128(vendor),
        part_type,
        product_date,
        b128(product_date),
        batch,
        b128(batch),
        ic,
        batch,
        product_date,
        vendor,
        qty,
        stop,
        user
    )
}

fn db() -> std::path::PathBuf {
    avc().join("jobs.db")
}

fn job_into_sqlite(ip: &str, job: &Job) -> i64 {
    let conn = rusqlite::Connection::open(db())
        .map_err(|error| {
            println!("{:?}", error);
            panic!("failure;")
        })
        .unwrap();

    // let mut stmt = conn.prepare("SELECT rowid, job FROM jobs").unwrap();

    let mut stmt = conn
        .prepare("INSERT INTO jobs (ip, job) values (?, ?)")
        .unwrap();

    let array = params![ip, serde_json::to_string(&job).unwrap()];

    match stmt.query_row(array, |row| Ok(row.get::<usize, String>(0)?)) {
        Ok(_) => (),
        Err(e) => match e {
            rusqlite::Error::QueryReturnedNoRows => (),
            e => panic!("bad insertion: {};", e),
        },
    };

    let mut stmt = conn.prepare("select last_insert_rowid();").unwrap();

    let array = params![];

    match stmt.query_row(array, |row| Ok(row.get::<usize, i64>(0).unwrap())) {
        Ok(s) => s,
        Err(e) => panic!("bad selection: {};", e),
    }
}

#[allow(dead_code)]
fn job_from_sqlite(i: &i64) -> (String, Job) {
    let conn = rusqlite::Connection::open(db())
        .map_err(|error| {
            println!("{:?}", error);
            panic!("failure;")
        })
        .unwrap();

    // let mut stmt = conn.prepare("SELECT rowid, job FROM jobs").unwrap();

    let mut stmt = conn
        .prepare("SELECT id, ip, job  from jobs where id = ?")
        .unwrap();

    let array = params![&i];

    match stmt.query_row(array, |row| {
        // println!("{:?}", row.get::<usize, i64>(0));
        // println!("{:?}", row.get::<usize, i64>(1));
        // println!("{:?}", row.get::<usize, i64>(2));
        Ok((
            row.get::<usize, String>(1).unwrap(),
            serde_json::from_str::<Job>(&row.get::<usize, String>(2).unwrap()).unwrap(),
        ))
    }) {
        Ok(s) => s,
        Err(e) => panic!("bad insertion: {};", e),
    }
}

fn job_last() -> i64 {
    let conn = rusqlite::Connection::open(db())
        .map_err(|error| {
            println!("{:?}", error);
            panic!("failure;")
        })
        .unwrap();

    // let mut stmt = conn.prepare("SELECT rowid, job FROM jobs").unwrap();

    let mut stmt = conn.prepare("SELECT MAX(rowid) from jobs;").unwrap();

    let array = params![];

    let max = stmt.query_row(array, |row| Ok(row.get::<usize, i64>(0).unwrap_or(0)));

    max.unwrap()
}

fn db_create() {
    let conn = rusqlite::Connection::open(db())
        .map_err(|error| {
            println!("{:?}", error);
            panic!("failure;")
        })
        .unwrap();

    // let mut stmt = conn.prepare("SELECT rowid, job FROM jobs").unwrap();

    conn.execute_batch(
        "create table if not exists 'jobs' (
        'id' integer primary key,
        'ip' text not null,
        'job' text not null,
        'ts' timestamp default (datetime('now', 'localtime'))
    );",
    )
    .unwrap();
}

fn jobs_from_sqlite(max: &i64, l: &i64) -> Vec<(i64, String, Job, String)> {
    let conn = rusqlite::Connection::open(db())
        .map_err(|error| {
            println!("{:?}", error);
            panic!("failure;")
        })
        .unwrap();

    // let mut stmt = conn.prepare("SELECT rowid, job FROM jobs").unwrap();

    let mut stmt = conn
        .prepare("select * from (SELECT id, ip, job, ts  from jobs where id <= ? limit ?) ORDER by id desc;")
        .unwrap();

    let array = params![&max, &l];

    let rows = stmt.query_map(array, |row| {
        // println!("{:?}", row.get::<usize, i64>(0));
        // println!("{:?}", row.get::<usize, i64>(1));
        // println!("{:?}", row.get::<usize, i64>(2));
        Ok((
            row.get::<usize, i64>(0).unwrap(),
            row.get::<usize, String>(1).unwrap(),
            serde_json::from_str::<Job>(&row.get::<usize, String>(2).unwrap()).unwrap(),
            row.get::<usize, String>(3).unwrap(),
        ))
    });

    let mut jobs = Vec::new();
    for row in rows.unwrap() {
        jobs.push(row.unwrap())
    }

    jobs
}

#[get(
    "/job?<jid>&<job>&<part>&<qty>&<start>&<stop>&<checksum>&<chip>&<vendor>&<part_type>&<date>&<lot>&<user>"
)]
fn job_get(
    jid: Option<i64>,
    job: Option<&RawStr>,
    part: Option<&RawStr>,
    qty: Option<u64>,
    start: Option<&RawStr>,
    stop: Option<&RawStr>,
    checksum: Option<&RawStr>,
    chip: Option<&RawStr>,
    vendor: Option<&RawStr>,
    part_type: Option<&RawStr>,
    date: Option<&RawStr>,
    lot: Option<&RawStr>,
    user: Option<&RawStr>,
) -> Markup {
    let jid = jid.unwrap_or(0);
    let job = job
        .unwrap_or_else(|| "WO119B0849".into())
        .url_decode()
        .unwrap();
    let part = part
        .unwrap_or_else(|| "A502000699-1".into())
        .url_decode()
        .unwrap();
    let qty = qty.unwrap_or_else(|| 5990);
    let start = start
        .unwrap_or_else(|| "2019-11-30 13:10:00".into())
        .url_decode()
        .unwrap();
    let stop = stop
        .unwrap_or_else(|| "2019-11-30 19:48:16".into())
        .url_decode()
        .unwrap();
    let checksum = checksum
        .unwrap_or_else(|| "7AF".into())
        .url_decode()
        .unwrap();
    let chip = chip.unwrap_or_else(|| "L036A".into()).url_decode().unwrap();
    let vendor = vendor
        .unwrap_or_else(|| "ALLEGRO".into())
        .url_decode()
        .unwrap();
    let part_type = part_type
        .unwrap_or_else(|| "A5931GES".into())
        .url_decode()
        .unwrap();
    let date = date
        .unwrap_or_else(|| "19/11/30".into())
        .url_decode()
        .unwrap();
    let lot = lot
        .unwrap_or_else(|| "1927693LBCA".into())
        .url_decode()
        .unwrap();
    let user = user
        .unwrap_or_else(|| "SZ123456".into())
        .url_decode()
        .unwrap();

    html! {
        h1 { "Job" }
        form action="/job" method="post" {

            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "jid" }; } input type="input" name="jid" value=(jid) readonly="readonly"; br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Job" }; } input type="input" name="job" value=(job); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Part" }; } input type="input" name="part" value=(part); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Qty" }; } input type="input" name="qty" value=(qty); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Start" }; } input type="input" name="start" value=(start); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Stop" }; } input type="input" name="stop" value=(stop); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Chip" }; } input type="input" name="chip" value=(chip); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Checksum" }; } input type="input" name="checksum" value=(checksum); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Vendor" }; } input type="input" name="vendor" value=(vendor); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Type" }; } input type="input" name="part_type" value=(part_type); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Date" }; } input type="input" name="date" value=(date); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "Lot" }; } input type="input" name="lot" value=(lot); br;
            div align="right" style=" margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px; " { span { "User" }; } input type="input" name="user" value=(user); br;

            input type="submit" value="Print";
        }

        p {a href="/" { "Home"}}

    }
}

use rocket::request::Form;
use rocket::request::FromForm;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, FromForm, Serialize, Deserialize)]
struct Job {
    jid: i64,
    job: String,
    part: String,
    qty: u64,
    start: String,
    stop: String,
    checksum: String,
    chip: String,
    vendor: String,
    part_type: String,
    date: String,
    lot: String,
    user: String,
}

impl Job {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            jid: 0,
            job: String::default(),
            part: String::default(),
            qty: 0,
            start: String::default(),
            stop: String::default(),
            checksum: String::default(),
            chip: String::default(),
            vendor: String::default(),
            part_type: String::default(),
            date: String::default(),
            lot: String::default(),
            user: String::default(),
        }
    }
}

#[get("/?<max>")]
fn home(max: Option<i64>) -> Markup {
    let lj = job_last();
    let max = max.unwrap_or(lj);

    let jobs = jobs_from_sqlite(&max, &10);
    html! {
        h1 {
            "jobs"
        }

        @for (id, ip, job, ts) in &jobs {
            p {
                b {( id )}; ", ";
                b {( ip )}; ", ";
                span {( job.jid )}; ", ";
                span {( job.job )}; ", ";
                span {( job.part )}; ", ";
                span {( job.qty )}; ", ";
                span {( job.start )}; ", ";
                span {( job.stop )}; ", ";
                span {( job.chip )}; ", ";
                span {( job.vendor )}; ", ";
                span {( job.part_type )}; ", ";
                span {( job.date )}; ", ";
                span {( job.lot )}; ", ";
                span {( job.user )}; ", ";
                span {( ts )}; ", ";

                a href=(format!("job?jid={}&job={}&part={}&qty={}&start={}&stop={}&checksum={}&chip={}&vendor={}&part_type={}&date={}&lot={}&user={}",
                 job.jid,
                 job.job,
                 job.part,
                 job.qty,
                 job.start,
                 job.stop,
                 job.checksum,
                 job.chip,
                 job.vendor,
                 job.part_type,
                 job.date,
                 job.lot,
                 job.user)) { "print" }
            }
        }

        //@if max != lj {
        a href="/job" { "New"}
        ", "
        a href="/" { "Home"}
        //}

        @if max > 10 {
            ", "
            a href=(format!("/?max={}", max-10)) { "next" }
        }
    }
    // <jid>&<job>&<part>&<qty>&<start>&<stop>&<chip>&<vendor>&<part_type>&<date>&<lot>&<user>
}

struct ThisUri(String);

impl<'a, 'r> FromRequest<'a, 'r> for ThisUri {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<ThisUri, ()> {
        println!("{:?}", request);
        request::Outcome::Success(ThisUri(request.client_ip().unwrap().to_string()))
    }
}

fn avc() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap().join("avc");
    std::fs::create_dir_all(&home).unwrap();
    home
}

fn ts() -> std::path::PathBuf {
    let now = chrono::Local::now();
    avc().join(format!("{}", now.format("%Y%m%d-%H%M%S.dmo")))
}

use std::fs::File;
use std::io::prelude::*;

fn tr_f(f: &std::path::PathBuf, s: &str) {
    let mut file = File::create(f).unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

#[post("/job", data = "<job>")]
fn job_post(job: Form<Job>, ip: ThisUri) -> Markup {
    let ip = &ip.0;
    let job = job.into_inner();

    tr_f(&ts(), &mark_job(&job));
    let id = job_into_sqlite(ip, &job);
    html! {
        h1 { "job: " (id) " - " "Printed!"}

        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "jid" } }; i {( job.jid )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Job" } }; i {( job.job )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Part" } }; i {( job.part )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Qty" } }; i {( job.qty )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Start" } }; i {( job.start )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Stop" } }; i {( job.stop )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Checksum" } }; i {( job.checksum )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Chip" } }; i {( job.chip )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Vendor" } }; i {( job.vendor )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Type" } }; i {( job.part_type )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Date" } }; i {( job.date )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "Lot" } }; i {( job.lot )}; br;
        div align="right" style="  margin-right:1em; margin-bottom: 6px; display: inline-block;width:100px;" { span { "User" } }; i {( job.user )}; br;

        p { a href="/" {"Home"} }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct Jinfo {
    device_name:Option<String>,
    checksum:Option<String>,
    quantity:Option<u32>,
    user_name:Option<String>,
    job_name:Option<String>,
    machine_name:Option<String>,
    model_name:Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct SbJob {
    fail: i64,
    jinfo:Option<Jinfo>,
    pass: i64,
    name: String,
    qty: i64,
    rowid: u64,
    start_at_t: u64,
    stop_at_t: u64,
    ts_t: u64,
    used: u64
}

#[derive(Debug, Serialize, Deserialize)]
struct LastError {
    code:u32,
    code_s:String,
    file:String,
    line:u32,
    db:u64,
    de:Option<u64>,
    last_error:serde_json::Value,
    last:Option<f32>
}

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    model:String,
    name:String,
    host:String,
    status:String,
    last_error:Option<LastError>
}


fn messagebox(msg: &str) -> Result<i32, std::io::Error> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::{MB_OK, MessageBoxW};
    let wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(once(0)).collect();
    let ret = unsafe {
        MessageBoxW(null_mut(), wide.as_ptr(), wide.as_ptr(), MB_OK)
    };
    if ret == 0 { Err(std::io::Error::last_os_error()) }
    else { Ok(ret) }
}

fn tray() -> Result<(), Box::<dyn std::error::Error>> {
    let mut app;
    match systray::Application::new() {
        Ok(w) => app = w,
        Err(_) => panic!("Can't create window!"),
    }
    // w.set_icon_from_file(&"C:\\Users\\qdot\\code\\git-projects\\systray-rs\\resources\\rust.ico".to_string());
    // w.set_tooltip(&"Whatever".to_string());
    let mut fbs = std::path::Path::new(&std::env::current_exe().unwrap()).parent().unwrap().join("busy.ico");
    if !fbs.exists() {
        fbs = std::path::Path::new("./").join("busy.ico");
    }
    fbs = std::fs::canonicalize(&fbs).unwrap();
    println!("fbs: {:?}", fbs);

    // app.set_icon_from_file("/usr/share/gxkb/flags/ua.png")?;
    let fbusy = std::rc::Rc::new(fbs);
    app.set_icon_from_file(&String::from(fbusy.as_ref().to_str().unwrap())).ok();
    
    app.add_menu_item("Home", |window| {
        println!("Home ...");

        let uri = format!("http://{}:{}/", 
                        "localhost", 
                        8000);
                        
        match open::that(uri) {
            Ok(_) => {
                
            },
            Err(e) => {
                messagebox(&format!("error: {}", e)).unwrap();
            }
        }
        Ok::<_, systray::Error>(())
    })?;

    app.add_menu_item("Print", |window| {    
        println!("Print ...");


        // Create the runtime
        let mut rt = Runtime::new().unwrap();

        // Spawn a future onto the runtime
        let block = rt.block_on(async {
            println!("now running on a worker thread");
            let client = hyper::Client::new();
            
            let res = client.get(format!("http://{}:{}/api/v3/json/query/machine_status", 
            "127.0.0.1", 
            8081).parse()?).await?;
            let slice = &hyper::body::to_bytes(res).await?;
            let json:serde_json::Value = serde_json::from_slice(slice)?;
            //println!("{:?}", slice);
            println!("{:?}", json);


            let res = client.get(format!("http://{}:{}/api/v3/json/query/statistics_job_info", 
            "127.0.0.1", 
            8081).parse()?).await?;
            let slice = &hyper::body::to_bytes(res).await?;
            let job_info:serde_json::Value = serde_json::from_slice(slice)?;
            //println!("{:?}", slice);
            println!("{:?}", job_info);



            let res = client.get(format!("http://{}:{}/api/v3/json/query/statistics_project_info", 
            "127.0.0.1", 
            8081).parse()?).await?;
            let slice = &hyper::body::to_bytes(res).await?;
            let project_info:serde_json::Value = serde_json::from_slice(slice)?;
            //println!("{:?}", slice);
            println!("{:?}", project_info);

            let res = client.get(format!("http://{}:{}/api/v3/json/query/statistics_session_info", 
            "127.0.0.1", 
            8081).parse()?).await?;
            let slice = &hyper::body::to_bytes(res).await?;
            let session_info:serde_json::Value = serde_json::from_slice(slice)?;
            //println!("{:?}", slice);
            println!("{:?}", session_info);
            
            let project_info = project_info.get("payload").ok_or(anyhow::anyhow!("no payload"))?.as_object().ok_or(anyhow::anyhow!("invalid payload"))?;
            let job_info = job_info.get("payload").ok_or(anyhow::anyhow!("no payload"))?.as_object().ok_or(anyhow::anyhow!("invalid payload"))?;

            let pinfo = project_info.get("pinfo").ok_or(anyhow::anyhow!("no pinfo"))?.as_object().ok_or(anyhow::anyhow!("invalid pinfo"))?;

            let job = Job {
                jid: job_info.get("rowid").ok_or(anyhow::anyhow!("no rowid"))?.as_i64().ok_or(anyhow::anyhow!("invalid rowid"))?,
                date: format!("{}", chrono::Local::now().format("%y/%m/%d")),
                job: String::from(""),
                part: pinfo.get("device").ok_or(anyhow::anyhow!("no device"))?.as_str().ok_or(anyhow::anyhow!("invalid device"))?.to_owned(),
                qty: job_info.get("qty").ok_or(anyhow::anyhow!("no qty"))?.as_i64().ok_or(anyhow::anyhow!("invalid qty"))? as u64,
                start: job_info.get("start_at").ok_or(anyhow::anyhow!("no start_at"))?.as_str().ok_or(anyhow::anyhow!("invalid start_at"))?.to_owned(),
                stop: job_info.get("stop_at").ok_or(anyhow::anyhow!("no stop_at"))?.as_str().ok_or(anyhow::anyhow!("invalid stop_at"))?.to_owned(),
                checksum: pinfo.get("checksum").ok_or(anyhow::anyhow!("no checksum"))?.as_str().ok_or(anyhow::anyhow!("invalid checksum"))?.to_owned(),
                chip: String::from(""), // pinfo.get("device").ok_or(anyhow::anyhow!("no device"))?.as_str().ok_or(anyhow::anyhow!("invalid device"))?.to_owned(),
                // vendor: pinfo.get("vendor").ok_or(anyhow::anyhow!("no vendor"))?.as_str().ok_or(anyhow::anyhow!("invalid vendor"))?.to_owned(),
                // part_type: pinfo.get("part_type").ok_or(anyhow::anyhow!("no part_type"))?.as_str().ok_or(anyhow::anyhow!("invalid part_type"))?.to_owned(),
                vendor: pinfo.get("vendor").unwrap_or(&serde_json::json!("no vendor"))?.as_str().ok_or(anyhow::anyhow!("invalid vendor"))?.to_owned(),
                part_type: pinfo.get("part_type").unwrap_or(&serde_json::json!("no part_type"))?.as_str().ok_or(anyhow::anyhow!("invalid part_type"))?.to_owned(),
                lot: String::from(""),
                user: String::from(""),
            };
            
    
            let uri =format!("http://{}:{}/job?jid={}&job={}&part={}&qty={}&start={}&stop={}&checksum={}&chip={}&vendor={}&part_type={}&date={}&lot={}&user={}",
                "localhost",
                8000,
                 job.jid,
                 job.job,
                 job.part,
                 job.qty,
                 job.start,
                 job.stop,
                 job.checksum,
                 job.chip,
                 job.vendor,
                 job.part_type,
                 job.date,
                 job.lot,
                 job.user);
                            
            open::that(uri)?;

            Ok::<(), Box::<dyn std::error::Error>>(())
        });

        if let Err(e) = block {
            messagebox(&format!("error: {}", e)).unwrap();
        }

        Ok::<_, systray::Error>(())
    })?;

    app.add_menu_separator()?;

    app.add_menu_item("Quit", |window| {
        window.quit();
        Ok::<_, systray::Error>(())
    })?;

    println!("Waiting on message!");
    app.wait_for_message()?;
    Ok(())
}

fn main() {
    std::thread::spawn( || {
        db_create();
        rocket::ignite()
            .mount("/", routes![home, job_get, job_post])
            .launch();
    });
    
    tray().unwrap();

}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_db() -> () {
        let j = Job::new();
        let i = "some ip".to_owned();
        //let u = "LL L".to_owned();
        db_create();

        assert_eq!(job_from_sqlite(&job_into_sqlite(&i, &j)), (i, j));

        let jobs = jobs_from_sqlite(&5, &4);
        for job in &jobs {
            println!("{:?}", job);
        }

        assert_eq!(jobs.len(), 4);
    }

    #[test]
    fn test_mark() -> () {
        assert_eq!(
            mark("0", "1", &2, "3", "4", "5", "6", "7", "8", "9", "10", "11"),
            "\x02L\r\n\
yUGB\r\n\
1911uC001800040P008P008B9A4B5A5BAC50000\r\n\
191100201800075: 0\r\n\
1e0202001600040B0\r\n\
1911uC001400040P008P008C1CFBAC50000\r\n\
191100301400065: 1\r\n\
1e0202001200040B1\r\n\
1911uC001400200P008P008CAFDC1BF0000\r\n\
191100201400225: 2\r\n\
1e0202001200200B2\r\n\
1911uC001000040P008P008BFAACABCCAB1BCE40000\r\n\
191100201000085: 3\r\n\
1911uC001000200P008P008BDE1CAF8CAB1BCE40000\r\n\
191100201000245: 4\r\n\
191100200800040CHECKSUM: 5\r\n\
1911uC000800200P008P008B3CCCABDB1E0C2EB0000\r\n\
191100200800245: 6\r\n\
1911uC000600040P008P008B3A7BCD20000\r\n\
191100200600065: 7\r\n\
1e0202000400040B7\r\n\
191100200600200TYPE: 8\r\n\
1911uC000200040P008P008D6C6D4ECC8D5C6DA0000\r\n\
191100200200090: 9\r\n\
1e0202000000040B9\r\n\
1911uC000200200P008P008B3F6B3A7C5FABAC50000\r\n\
191100200200250: 10\r\n\
1e0202000000200B10\r\n\
1W1d4400001300300P1;L10;D9;V7;Q2;R4;U11\r\n\
\r\n\
E\r\n\
"
        );
    }
}
