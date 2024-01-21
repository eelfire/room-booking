use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// #[derive(Debug)]
// struct Request {
//     time: u32,
//     req_type: RequestType,
//     room: Option<u32>,
//     timeslot: Option<String>,
// }

#[derive(Debug)]
enum RequestType {
    Book,
    Cancel,
    Get,
}

#[derive(Debug)]
enum ResponseStatus {
    OK = 0,
    SlotNotAvailable = -1,
    CooldownPeriod = -2,
    InvalidRequest = -3,
}

#[derive(Debug)]
struct Slot {
    time: String,
    // length: u32,
    booked: bool,
    booked_at: u32,
}

#[derive(Debug)]
struct Room {
    id: u32,
    timeslots: Vec<Slot>,
}

#[derive(Debug)]
struct RoomsState {
    rooms: Vec<Room>,
}

#[derive(Debug)]
struct Booked {
    room: u32,
    timeslot: String,
}

fn u8_array_to_string(bytes: &[u8]) -> String {
    let mut string = String::new();
    for byte in bytes {
        string.push(*byte as char);
    }
    string
}

// fn string_to_u8_array(string: String) -> [u8; 1024] {
//     let mut bytes = [0; 1024];
//     for (i, byte) in string.bytes().enumerate() {
//         bytes[i] = byte;
//     }
//     bytes
// }

async fn process_request(
    _stream: &mut TcpStream,
    buffer: &[u8],
    rooms_state: &mut RoomsState,
) -> Vec<u8> {
    let request_str = u8_array_to_string(buffer);
    // println!("request: {}", request_str);

    let split_request: Vec<&str> = request_str.split(',').collect();
    println!("split request: {:?}", split_request);

    let [time, req_type, room, timeslot] = split_request[..] else {
        panic!("invalid request")
    };
    let timeslot = timeslot
        .split(|c| c == '\r' || c == '\n')
        .collect::<Vec<&str>>()[0];
    // println!("time: {}", time);

    let req_type = match req_type {
        "BOOK" => RequestType::Book,
        "CANCEL" => RequestType::Cancel,
        "GET" => RequestType::Get,
        _ => panic!("invalid request type"),
    };

    let mut get_booked = vec![];

    let response_status = match req_type {
        RequestType::Book => {
            let time = time.parse::<u32>().unwrap();
            let room = room.parse::<u32>().unwrap();
            let timeslot = timeslot.to_string();
            // dbg!(&time, &room, &timeslot);
            let r_room = room.clone();

            let room = rooms_state.rooms.iter_mut().find(|r| r.id == room);
            if let Some(room) = room {
                let slot = room.timeslots.iter_mut().find(|slot| slot.time == timeslot);
                if let Some(slot) = slot {
                    if slot.booked {
                        ResponseStatus::SlotNotAvailable
                    } else {
                        slot.booked = true;
                        slot.booked_at = time;
                        ResponseStatus::OK
                    }
                } else {
                    // ResponseStatus::InvalidRequest
                    let timeslot = Slot {
                        time: timeslot,
                        booked: true,
                        booked_at: time,
                    };
                    room.timeslots.push(timeslot);
                    ResponseStatus::OK
                }
            } else {
                // ResponseStatus::InvalidRequest
                let slot = Slot {
                    time: timeslot,
                    booked: true,
                    booked_at: time,
                };
                let room = Room {
                    id: r_room,
                    timeslots: vec![slot],
                };
                rooms_state.rooms.push(room);
                ResponseStatus::OK
            }
        }
        RequestType::Cancel => {
            let time = time.parse::<u32>().unwrap();
            let room = room.parse::<u32>().unwrap();
            let timeslot = timeslot.to_string();

            let room = rooms_state.rooms.iter_mut().find(|r| r.id == room);
            if let Some(room) = room {
                let slot = room.timeslots.iter_mut().find(|slot| slot.time == timeslot);
                if let Some(slot) = slot {
                    if slot.booked {
                        if time - slot.booked_at < 20 {
                            ResponseStatus::CooldownPeriod
                        } else {
                            slot.booked = false;
                            ResponseStatus::OK
                        }
                    } else {
                        ResponseStatus::InvalidRequest
                    }
                } else {
                    ResponseStatus::InvalidRequest
                }
            } else {
                ResponseStatus::InvalidRequest
            }
        }
        RequestType::Get => {
            rooms_state.rooms.iter().for_each(|room| {
                room.timeslots.iter().for_each(|slot| {
                    if slot.booked {
                        let booked_slot = Booked {
                            room: room.id,
                            timeslot: slot.time.clone(),
                        };
                        get_booked.push(booked_slot);
                    }
                });
            });
            ResponseStatus::OK
        }
    };

    println!("response status: {:?}", response_status);

    // let response = "2,BOOK,1,14:00-15:30,0\r\n";
    // eg get response: 10,GET,,,0,"{('5', '15:30-17:00'), ('5', '08:00-09:30'), ('1', '15:30-17:00'), ('3', '12:30-14:00'), ('1', '09:30-11:00'), ('4', '17:00-18:30')}"
    let mut response = String::new();
    response.push_str(time);
    response.push(',');
    response.push_str(match req_type {
        RequestType::Book => "BOOK",
        RequestType::Cancel => "CANCEL",
        RequestType::Get => "GET",
    });
    response.push(',');
    response.push_str(room);
    response.push(',');
    response.push_str(timeslot);
    response.push(',');
    response.push_str((response_status as i32).to_string().as_str());
    response.push(',');

    if let RequestType::Get = req_type {
        println!("get booked: {:?}", get_booked);
        response.push_str("\"{");
        for (idx, booked) in get_booked.iter().rev().enumerate() {
            response.push('(');
            response.push_str(format!("'{}', ", booked.room).as_str());
            response.push_str(format!("'{}'", booked.timeslot).as_str());
            response.push(')');
            if idx != get_booked.len() - 1 {
                response.push(',');
                response.push(' ');
            }
        }
        response.push_str("\"}");
    }

    response.push('\r');
    response.push('\n');

    response.as_bytes().to_vec()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("hello from the server!");

    let listener = TcpListener::bind("0.0.0.0:1234")
        .await
        .expect("Failed to bind to address");

    while let Ok((stream, _)) = listener.accept().await {
        println!("new client connected");
        let mut rooms_state = RoomsState { rooms: Vec::new() };

        tokio::spawn(async move {
            let mut stream = stream;
            let mut buffer = [0; 1024];

            while let Ok(bytes_read) = stream.read(&mut buffer).await {
                if bytes_read == 0 {
                    break;
                }

                let response =
                    process_request(&mut stream, &buffer[..bytes_read], &mut rooms_state).await;
                stream.write_all(response.as_slice()).await.unwrap();
            }
        });
    }

    Ok(())
}
