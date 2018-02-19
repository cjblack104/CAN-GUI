extern crate libc;
extern crate rustc_serialize;
extern crate can_gui;
use libc::*;
use can_gui::ThreadPool;

//To hide console window
#[windows_subsystem = "windows"]

const ERROR_OK: i16 = 0;
//static mut hndl: i16 = 0;

#[link(name = "canlib32")]
extern {
	fn canOpenChannel(ctrl: u16, flags: u16) -> i16;
	fn canInitializeLibrary();
	fn canSetBusParams(handle: i16, bitrate: i32, tseg1: u16, tseg2: u16, sjw: u16, noSamp: u16, syncmode: u16) -> i16;
	fn canWriteWait(handle: i16, id: u32, msg: *const c_void, dlc: u16, flag: u16, timeout: u32) -> i16;
	fn canWrite(handle: i16, id: u32, msg: *const c_void, dlc: u16, flag: u16) -> i16;
	fn canBusOn(handle: i16) -> i16;
	fn canClose(handle: i16) -> i16;
	//fn canReadWait(handle: i16, id: *mut i32, msg: *mut c_void, dlc: *mut u16, flag: *mut u16, time: *mut u32, timeout: u32) -> i16;
	//fn canReadSyncSpecific(handle: i16, id: u16, timeout: u32) -> i16;
	fn canReadSpecificSkip(handle: i16, id: i32, msg: *mut c_void, dlc: *mut u16, flag: *mut u16, time: *mut u32) -> i16;
	fn canFlushReceiveQueue(handle: i16) -> i16;
}

use common::SOCKET_PATH;

use std::thread;
use std::net::TcpListener;


fn main() {
	// CAN library initialization
	unsafe {canInitializeLibrary()};

	let mut hndl : i16;

	// Initialize TCP Listener
	let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
	let pool = ThreadPool::new(4);

	// Consider each input from the UI as a unique request
	for stream in listener.incoming().take(2) {
		let stream = stream.unwrap();

		// Multi-thread that shit
		pool.execute(|| {
			handle_connection(stream);
		});
	}

	// Flush queue to be safe
	unsafe{canFlushReceiveQueue(hndl)};
	
	// Close bus
	let result = unsafe {canClose(hndl)};
		if result != ERROR_OK {
		println!("Failed to close bus. Error: {}", result);
		return
	}
}

fn handle_connection(mut stream: TcpStream) {
	let mut buffer = [0; 512];
	stream.read(&mut buffer).unwrap();

	// todo: Define protocol to use with TCP
	// Each unique request needs its own ID to be checked in the If statement (possibly make this a match statement)
	// ID corresponds to UI element
	let get = b"GET / HTTP/1.1\r\n";
	let sleep = b"GET /sleep HTTP/1.1\r\n";

	if buff.starts_with(get) {
		("HTTP/1.1 200 OK\r\n\r\n", "hello,html")
	} else if buffer.starts_with(sleep) {
		thread::sleep(Duration::from_secs(5));
		("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
	} else {
		("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
	};

	let mut file = File::open(filename).unwrap();
	let mut contents = String::new();

	file.read_to_string(&mut contents).unwrap();

	let response = format!("{}{}", status_line, contents);

	stream.write(response.as_bytes()).unwrap();
	stream.flush().unwrap();
}

fn start_can(bus : u8, bitrate : i32, mut hndl : i16) {
	unsafe {
		hndl = canOpenChannel(bus, 0);
		if hndl < ERROR_OK {
			println!("Failed to open CAN channel. Error: {}", hndl);
			return
		}
	}
	let mut result = unsafe {canSetBusParams(hndl, bitrate, 0, 0, 0, 0, 0)};

	if result != ERROR_OK {
		println!("Failed to set CAN bus parameters. Error: {}", result);
		return
	}

	result = unsafe {canBusOn(hndl)};
	if result != ERROR_OK {
		println!("Failed to go on bus. Error: {}", result);
		return
	}
	let response = format!("1 0");
	stream.write(response.as_bytes()).unwrap();
	stream.flush().unwrap();
}

fn timekeeper(bus : u8, mut hndl : i16, id : i32, data : void, dlc : u8) {
	//todo: Some timer stuff I guess
	// maybe do this: https://www.reddit.com/r/rust/comments/4nvuwc/periodic_timer_in_rust/d47c1s4/
}

// Call this in timekeeper
// Pass all message data from TCP Port into timekeeper, then here
// Message Args: {canWriteWait(hndl, id, data, dlc, flag, timeout)}
fn send_messages(bus : u8, mut hndl : i16, id : i32, data : void, dlc : u8) {
	let result = unsafe {canWriteWait(hndl, id, data.as_mut_ptr() as *mut c_void, dlc, 1, 10000)};
	if result != ERROR_OK {
		println!("Torque Message Fucked Up. Error: {}", result);
		return
	}
}