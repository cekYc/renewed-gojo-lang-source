
#![allow(dead_code, unused_imports, unused_variables, unused_parens, unused_mut)]
use std::time::Duration;
use std::io::{self, Write};
use serde_json::Value;

const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";   
const GREEN: &str = "\x1b[32m";  
const MAGENTA: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m"; 
const BLUE: &str = "\x1b[34m";
const RED: &str = "\x1b[31m";

struct DB;
impl DB {
    async fn log<T: std::fmt::Display>(msg: T) { println!("  {}[DB] Log: {}{}", CYAN, msg, RESET); }
}

struct Console;
impl Console {
    async fn read(prompt: String) -> String {
        print!("  {}[Console] 👂 {}: {} ", BLUE, prompt, RESET);
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        buffer.trim().to_string()
    }
}

struct Util;
impl Util {
    #[inline(always)]
    async fn to_int(s: String) -> i64 { s.trim().parse::<i64>().unwrap_or(0) }
    #[inline(always)]
    async fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }
}

struct HTTP;
impl HTTP {
    async fn get(url: String) -> String {
        let client = reqwest::Client::builder().user_agent("GojoLang/1.0").build().unwrap();
        match client.get(&url).send().await {
            Ok(res) => res.text().await.unwrap_or_else(|e| format!("Error: {}", e)),
            Err(e) => format!("Error: {}", e)
        }
    }
}

trait Validate { fn validate(&self) -> Result<String, String>; }
impl Validate for String {
    fn validate(&self) -> Result<String, String> { Ok(self.clone()) }
}

// TRAITLER (Inline optimize edildi)
trait GojoAdd<Rhs> { type Output; fn g_add(self, rhs: Rhs) -> Self::Output; }
impl GojoAdd<i64> for i64 { type Output = i64; #[inline(always)] fn g_add(self, rhs: i64) -> i64 { self + rhs } }
impl GojoAdd<String> for String { type Output = String; #[inline(always)] fn g_add(self, rhs: String) -> String { self + &rhs } }
impl<'a> GojoAdd<&'a str> for String { type Output = String; #[inline(always)] fn g_add(self, rhs: &'a str) -> String { self + rhs } }
impl GojoAdd<i64> for String { type Output = String; #[inline(always)] fn g_add(self, rhs: i64) -> String { format!("{}{}", self, rhs) } }

trait GojoMul<Rhs> { type Output; fn g_mul(self, rhs: Rhs) -> Self::Output; }
impl GojoMul<i64> for i64 { type Output = i64; #[inline(always)] fn g_mul(self, rhs: i64) -> i64 { self * rhs } }
impl GojoMul<i64> for String { type Output = String; fn g_mul(self, rhs: i64) -> String { self.repeat(rhs as usize) } }
impl<'a> GojoMul<i64> for &'a str { type Output = String; fn g_mul(self, rhs: i64) -> String { self.repeat(rhs as usize) } }
pub fn fib(n: i64) -> i64 {
    if (n <= 1) {
        return n;
    }
    let mut a = fib((n - 1));
    let mut b = fib((n - 2));
    return (a + b);
}

pub async fn user_main(girdi: String) -> () {
    let girdi = girdi.validate().unwrap();
    // Scope: Benchmark
    {
        let mut n = 40;
        tokio::spawn(async move { DB::log("Gojo: fib(".to_string().g_add(n).g_add(") hesaplaniyor...".to_string())).await });
        let mut start = Util::now().await;
        let mut sonuc = fib(n);
        let mut end = Util::now().await;
        tokio::spawn(async move { DB::log("Sonuc: ".to_string().g_add(sonuc)).await });
        tokio::spawn(async move { DB::log("Gecen Sure: ".to_string().g_add((end - start)).g_add(" ms".to_string())).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::main] async fn main() { user_main("Internet".to_string()).await; tokio::time::sleep(std::time::Duration::from_millis(100)).await; }