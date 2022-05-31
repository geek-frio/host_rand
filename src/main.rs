use clap::Parser;
use rand::prelude::*;
use regex::Regex;
use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::{BufWriter, Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    host: String,
}

fn comment_all(buf: &mut Vec<String>) {
    buf.iter_mut()
        .filter(|a| !check_comment_prefix(*a))
        .for_each(|a| {
            a.insert_str(0, "#");
        });
}

fn check_comment_prefix(s: &str) -> bool {
    let mut chs = s.chars();
    while let Some(c) = chs.next() {
        match c {
            ' ' => continue,
            '#' => return true,
            _ => return false,
        }
    }
    false
}

fn choose_one(buf: &mut Vec<String>, black_list: &mut HashSet<String>) -> String {
    let mut thread_rng = thread_rng();
    loop {
        let idx = thread_rng.gen_range(0..buf.len());
        let s = &buf[idx];
        let ip = s.clone();
        if black_list.contains(s) {
            continue;
        } else {
            uncomment_prefix(buf, idx);
            return ip;
        }
    }
}

fn uncomment_prefix(buf: &mut Vec<String>, idx: usize) {
    let s = buf.get_mut(idx).expect("get comment host failed");
    if check_comment_prefix(s) {
        let chs = s.chars().into_iter();
        let mut flag = false;
        let s = chs
            .filter(|a| {
                if !flag {
                    if *a == '#' {
                        flag = true;
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            })
            .collect::<String>();
        println!("s is: {}", s);
        buf[idx] = s;
    }
}

fn re_select(buf: &mut Vec<String>, black_list: &mut HashSet<String>) -> String {
    comment_all(buf);
    choose_one(buf, black_list)
}

fn group_host_file<'a>(host: &'a str, s: &'a String) -> (Vec<String>, Vec<String>) {
    let lines = s.split('\n');
    let mut hosts: Vec<String> = Vec::new();
    let not_hosts: Vec<String> = lines
        .filter(|a| {
            if a.contains(&host) {
                hosts.push(a.to_string());
                return false;
            } else {
                true
            }
        })
        .map(|a| a.to_string())
        .collect();
    (not_hosts, hosts)
}

fn flush(not_hosts: &Vec<String>, hosts: &Vec<String>) {
    let file = OpenOptions::new()
        .write(true)
        .open("/etc/hosts")
        .expect("打开/etc/hosts文件写入失败");
    let chain = not_hosts.into_iter().chain(hosts.into_iter());
    let mut buf_write = BufWriter::new(file);
    for h in chain {
        let _ = buf_write.write_fmt(format_args!("{}\n", h));
    }
    let _ = buf_write.flush();
    println!("Host file is updated!");
}

fn read_hosts_content() -> String {
    let mut file = OpenOptions::new()
        .read(true)
        .open("/etc/hosts")
        .expect("编辑 /etc/hosts文件 失败,没有权限！");
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .expect("/etc/hosts 文件内容读取失败！");
    buf
}

fn watcher_processsor(send: Sender<Event>, host: String) {
    std::thread::spawn(move || {
        let mut counter = 0;
        loop {
            let ip = get_current_inuse(host.as_str());
            match ip {
                Some(ip) => {
                    let (sta, cost) = try_to_start_tcp_conn(&ip);
                    if !sta {
                        counter += 1;
                        if counter >= 3 {
                            counter = 0;
                            println!("多次尝试仍不能正常连接,尝试随机切换host对应的ip...");
                            let _ = send.send(Event::Default(ip));
                        } else {
                            println!("Host绑定对应ip出现超时情况,再次尝试, 尝试次数:{}!", counter);
                        }
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    } else if cost >= 500 {
                        let mut total_cost = 0;
                        for _ in 0..3 {
                            let (_, cost) = try_to_start_tcp_conn(&ip);
                            total_cost += cost;
                        }
                        if total_cost / 3 >= 500 {
                            let _ = send.send(Event::Default(ip));
                        }
                    }
                }
                None => {
                    println!("Host is not in use, send default event....");
                    let _ = send.send(Event::Default("".to_string()));
                }
            }
            std::thread::sleep(Duration::from_secs(10))
        }
    });
}

fn try_to_start_tcp_conn(ip: &str) -> (bool, u128) {
    let conn_str = format!("{}:443", ip);
    let current = Instant::now();
    let sock_addr = conn_str
        .to_socket_addrs()
        .unwrap()
        .collect::<Vec<SocketAddr>>();
    let res = TcpStream::connect_timeout(sock_addr.get(0).unwrap(), Duration::from_secs(1));
    match res {
        Ok(s) => {
            println!(
                "Host 对应ip: {} 连接正常, 连接耗时:{}ms",
                ip,
                current.elapsed().as_millis()
            );
            drop(s);
            let elapsed = current.elapsed().as_millis();
            if current.elapsed().as_millis() > 500 {
                (false, elapsed)
            } else {
                (true, elapsed)
            }
        }
        Err(e) => {
            println!("Have met error in connecting..., e:{:?}", e);
            let elapsed = current.elapsed().as_millis();
            (false, elapsed)
        }
    }
}

fn get_current_inuse(host: &str) -> Option<String> {
    let content = read_hosts_content();
    let linest = content.split('\n');
    let re = Regex::new(r"\d+\.\d+\.\d+\.\d+").unwrap();

    for line in linest {
        if line.contains(&host) && !line.starts_with("#") {
            let s = re.captures(line).unwrap().get(0).unwrap();
            return Some(s.as_str().to_string());
        }
    }
    None
}

enum Event {
    Default(String),
}

fn main() {
    let args = Args::parse();

    let (send, recv) = std::sync::mpsc::channel::<Event>();

    watcher_processsor(send, args.host.clone());
    loop {
        let event = recv.recv().unwrap();
        match event {
            Event::Default(black_ip) => {
                let content = read_hosts_content();
                let (not_hosts, mut hosts) = group_host_file(args.host.as_str(), &content);
                let mut black_list = HashSet::new();
                if black_ip.len() > 0 {
                    black_list.insert(black_ip);
                }
                let ip = re_select(&mut hosts, &mut black_list);
                flush(&not_hosts, &hosts);
                println!("重新选择的ip地址是:{}", ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basics() {
        let cont = read_hosts_content();
        println!("cont:{}", cont);
        let (_not_hosts, mut hosts) = group_host_file("zuozhu163.xyz", &cont);
        // println!("not_hosts: {:?}, hosts:{:?}", not_hosts, hosts);

        comment_all(&mut hosts);
        // for h in &not_hosts {
        //     println!("{}", h);
        // }
        // for h in &hosts {
        //     println!("{}", h);
        // }

        let mut black_list = HashSet::new();
        choose_one(&mut hosts, &mut black_list);
        for _h in &hosts {
            // println!("{}", h);
        }
        println!("aaaaa{:?}", get_current_inuse("zuozhu163.xyz"));
    }

    #[test]
    fn test_watcher() {
        println!("result is:{:?}", try_to_start_tcp_conn("103.21.247.112"));
    }
}
