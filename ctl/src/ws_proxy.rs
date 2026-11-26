use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

fn parse_ws_handshake(reader: &mut BufReader<&mut TcpStream>) -> Option<(String, String)> {
    let mut req_line = String::new();
    reader.read_line(&mut req_line).ok()?;
    let path = req_line.split_whitespace().nth(1)?;

    let target = path
        .split('?')
        .nth(1)?
        .split('&')
        .find_map(|pair| pair.strip_prefix("target="))?
        .to_string();

    let mut ws_key = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                ws_key = Some(v.trim().to_string());
            }
        }
    }

    let key = ws_key?;
    let concat = format!("{key}{WS_GUID}");
    let digest = crate::sha1::sha1(concat.as_bytes());
    let accept = crate::b64::encode(&digest);
    Some((target, accept))
}

fn read_ws_frame(reader: &mut BufReader<&mut TcpStream>) -> Option<Vec<u8>> {
    let mut hdr = [0u8; 2];
    reader.read_exact(&mut hdr).ok()?;
    let opcode = hdr[0] & 0x0f;
    if opcode == 0x08 {
        // Close frame
        return None;
    }

    let masked = (hdr[1] & 0x80) != 0;
    let mut payload_len = (hdr[1] & 0x7f) as usize;

    if payload_len == 126 {
        let mut ext = [0u8; 2];
        reader.read_exact(&mut ext).ok()?;
        payload_len = u16::from_be_bytes(ext) as usize;
    } else if payload_len == 127 {
        let mut ext = [0u8; 8];
        reader.read_exact(&mut ext).ok()?;
        payload_len = u64::from_be_bytes(ext) as usize;
    }

    let mask_key = if masked {
        let mut m = [0u8; 4];
        reader.read_exact(&mut m).ok()?;
        Some(m)
    } else {
        None
    };

    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload).ok()?;

    if let Some(m) = mask_key {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= m[i % 4];
        }
    }

    if opcode == 0x09 {
        // Ping frame: payload can be ignored for proxying
        return Some(Vec::new());
    }

    Some(payload)
}

fn write_ws_frame(stream: &mut TcpStream, data: &[u8]) -> bool {
    let mut hdr = Vec::with_capacity(10);
    hdr.push(0x82); // FIN + Binary frame
    let len = data.len();
    if len < 126 {
        hdr.push(len as u8);
    } else if len <= 0xffff {
        hdr.push(126);
        hdr.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        hdr.push(127);
        hdr.extend_from_slice(&(len as u64).to_be_bytes());
    }

    if stream.write_all(&hdr).is_err() {
        return false;
    }
    if stream.write_all(data).is_err() {
        return false;
    }
    stream.flush().is_ok()
}

fn handle_connection(mut client: TcpStream) {
    let _ = client.set_read_timeout(Some(Duration::from_secs(10)));
    let (target, accept) = {
        let mut reader = BufReader::new(&mut client);
        let Some(pair) = parse_ws_handshake(&mut reader) else {
            return;
        };
        pair
    };

    let Ok(addrs) = target.to_socket_addrs() else {
        let _ = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nBad target address");
        return;
    };

    let mut tcp_target = None;
    for addr in addrs {
        if let Ok(s) = TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            tcp_target = Some(s);
            break;
        }
    }

    let Some(target_stream) = tcp_target else {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\nFailed to connect to target");
        return;
    };

    let resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    if client.write_all(resp.as_bytes()).is_err() {
        return;
    }
    let _ = client.flush();

    // Reset timeouts for streaming
    let _ = client.set_read_timeout(None);
    let _ = target_stream.set_read_timeout(None);

    let client_read = client.try_clone().ok();
    let mut client_write = client;
    let target_read = target_stream.try_clone().ok();
    let mut target_write = target_stream;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_c = Arc::clone(&stop);

    // Thread 1: WebSocket -> Target TCP
    let h1 = thread::spawn(move || {
        if let Some(mut c_stream) = client_read {
            let mut reader = BufReader::new(&mut c_stream);
            while !stop_c.load(Ordering::Relaxed) {
                match read_ws_frame(&mut reader) {
                    Some(payload) => {
                        if !payload.is_empty() && target_write.write_all(&payload).is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
        stop_c.store(true, Ordering::Relaxed);
    });

    // Thread 2: Target TCP -> WebSocket
    let stop_t = Arc::clone(&stop);
    let h2 = thread::spawn(move || {
        if let Some(mut t_stream) = target_read {
            let mut buf = [0u8; 16384];
            while !stop_t.load(Ordering::Relaxed) {
                match t_stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if !write_ws_frame(&mut client_write, &buf[..n]) {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
        stop_t.store(true, Ordering::Relaxed);
    });

    let _ = h1.join();
    let _ = h2.join();
}

pub fn run_wsproxy(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .map_err(|e| format!("failed to bind wsproxy on 127.0.0.1:{port}: {e}"))?;
    println!("wsproxy_port={port}");
    println!("wsproxy_running=1");

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            handle_connection(stream);
        });
    }
    Ok(())
}
