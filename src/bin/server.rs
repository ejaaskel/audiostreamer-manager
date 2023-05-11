//! A mDNS query client.
//!
//! Run with:
//!
//!     cargo run --example query <service_type_without_domain>
//!
//! Example:
//!
//!     cargo run --example query _my-service._udp
//!
//! Note: there is no '.' at the end as the program adds ".local."
//! automatically.
//!
//! Keeps listening for new events.

use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::{env, process, thread, time::Duration, collections::HashMap};

fn create_address_port_string(address: String, port: u16) -> String {
    let mut address_port_string = String::new();
    address_port_string.push_str(&address);
    address_port_string.push_str(":");
    address_port_string.push_str(&port.to_string());
    address_port_string
}

fn concatenate_address_strings(addresses: Vec<String>) -> String {
    let mut address_string = String::new();
    for address in addresses {
        address_string.push_str(&address);
        address_string.push_str(",");
    }
    //Remove last comma from address string
    address_string.pop();
    address_string
}

/*fn startGstProcess(address_string: String, gst_proc: &mut std::process::Child) {
    let mut clients_string = String::new();
    clients_string.push_str("clients=");
    clients_string.push_str(&address_string);

    //TODO - create a variable that can shut down gst_proc
    match gst_proc.try_wait() {
        Ok(Some(status)) => println!("exited with: {}", status),
        Ok(None) => {
            println!("status not ready yet, let's really wait");
            gst_proc.kill().expect("command wasn't running");
            let res = gst_proc.wait();
            println!("result: {:?}", res);
        }
        Err(e) => println!("error attempting to try wait: {}", e),
    }

    gst_proc = Command::new("gst-launch-1.0")
                           .arg("alsasrc device=hw:2,1,1 !")
                           .arg("audioconvert !")
                           .arg("audioresample !")
                           .arg("rtpL16pay !")
                           .arg("multiudpsink")
                           .arg(clients_string)
                           .spawn();
}*/

fn main() {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    let mut service_type = match env::args().nth(1) {
        Some(arg) => arg,
        None => {
            println!("ERROR: require a service_type as argument. For example: ");
            println!("cargo run --example query _my-service._udp");
            return;
        }
    };

    service_type.push_str(".local.");
    let receiver = mdns.browse(&service_type).expect("Failed to browse");
    let mut speaker_services = HashMap::new();

    let (tx, rx) = std::sync::mpsc::channel();

    let _thread = std::thread::spawn(move || {
        loop {
            thread::park();
            println!("Thread unparked");
            let received = rx.recv().unwrap();
            for (key, value) in &received {
                println!("Service {}: {}", key, value);
            }
        }
    });

    let now = std::time::Instant::now();
    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                println!(
                    "At {:?}: Resolved a new service: {} host: {} port: {} IP: {:?} TXT properties: {:?}",
                    now.elapsed(),
                    info.get_fullname(),
                    info.get_hostname(),
                    info.get_port(),
                    info.get_addresses(),
                    info.get_properties(),
                );
                // This works on a bold assumption that there will be only one address
                speaker_services.insert(String::from(info.get_fullname()), 
                                                     info.get_addresses()
                                                        .iter()
                                                        .next()
                                                        .expect("No addresses")
                                                        .to_string());
                tx.send(speaker_services.clone()).unwrap();
                _thread.thread().unpark();
            }
            ServiceEvent::ServiceRemoved(service_type, fullname) => {
                println!(
                    "At {:?}: Removed: service_type: {} fullname: {}",
                    now.elapsed(),
                    service_type,
                    fullname
                );
                speaker_services.remove(&fullname);
                tx.send(speaker_services.clone()).unwrap();
                _thread.thread().unpark();
            }
            other_event => {
                println!(
                    "At {:?} : Received other event: {:?}",
                    now.elapsed(),
                    &other_event
                );
            }
        }
    }
}
