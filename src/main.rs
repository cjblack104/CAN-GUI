extern crate libc;
extern crate rustc_serialize;
extern crate can_gui;
use libc::*;
use can_gui::ThreadPool;
use std::thread;
use std::net::TcpStream;
use std::net::TcpListener;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::io::Read;
use std::io::Write;

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

	// todo: Define protocol to use with TCP (because we're not using HTTP, surprise)
	// Each unique request needs its own ID to be checked in the If statement (possibly make this a match statement)
	// ID corresponds to UI element
	// Let's do something like Function BusID Bitrate CANID DLC MessageData
	// Function determines the presence of following parameters(?)
	// Functions Timekeeper, StartCAN, SendMessage
	let start_timer = b"GET Timekeeper\r\n";
	let send_message = b"GET SendMessage\r\n";
	let setup_can = b"GET StartCAN\r\n";
	let (tx : Sender<T>, rx : Receiver<T>) = channel();				// to start and stop timekeeper

	//println!("Message from Port 8080: {}", buffer);

	if buffer.starts_with(start_timer) {
		//timekeeper(rx, bus, hndl, id, data, dlc);
	} else if buffer.starts_with(setup_can){
		//start_can(bus, bitrate, hndl);
	} else {
		//error
	};

	// respond with a confirmation
	//let response = format!("{}{}", status_line, contents);

	//stream.write(response.as_bytes()).unwrap();
	stream.flush().unwrap();
}

fn start_can(bus : u16, bitrate : i32, mut hndl : i16) {
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
}

fn timekeeper(rx : Receiver<()>, bus : u16, mut hndl : i16, id : u32, mut data : [u8; 8], dlc : u16) {
	// todo: Some timer stuff I guess
		// execute code once a second, receive stop timer command via rx
	while rx.recv().unwrap().eq(0) {
		thread::sleep(Duration::from_secs(1));
		send_messages(bus, hndl, id, data, dlc);
	}
}

// Call this in timekeeper
// Pass all message data from TCP Port into timekeeper, then here
// Message Args: {canWriteWait(hndl, id, data, dlc, flag, timeout)}
fn send_messages(bus : u16, mut hndl : i16, id : u32, mut data : [u8; 8], dlc : u16) {
	let result = unsafe {canWriteWait(hndl, id, data.as_mut_ptr() as *mut c_void, dlc, 1, 10000)};
	if result != ERROR_OK {
		println!("Message Fucked Up. Error: {}", result);
		return
	}
}