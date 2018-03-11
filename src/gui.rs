extern crate libc;
extern crate rustc_serialize;
use libc::*;

//To hide console window
#[windows_subsystem = "windows"]

#[macro_use] extern crate native_windows_gui as nwg;

const ERROR_OK: i16 = 0;
static mut hndl: i16 = 0;	

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

// Possibly not needed
// define struct for use with Battery CAN
#[derive(RustcEncodable)]
struct Cell {
	id: u32,
	data: [u8; 8],
	length: u16,
	flag: u16,
	time: u32,
}

use nwg::{Event, Ui, simple_message, fatal_message, dispatch_events, exit as nwg_exit};
use nwg::constants::{FONT_WEIGHT_BLACK, FONT_DECO_ITALIC, CheckState, FileDialogAction, HTextAlign, PickerDate};

/// Custom enums are the preferred way to define ui ids. It's clearer and more extensible than any other types (such as &'str).
#[derive(Debug, Clone, Hash)]
pub enum AppId {
    // Controls
    MainWindow,
	StartCANButton,
	BusSelect,
	ModeSelect,
	TimeKeeper,
	SendMessagesButton,
	Bus0Radio,
	Bus1Radio,
    Label(u8),   // Ids for static controls that won't be referenced in the Ui logic can be shortened this way.
	TorqueStepInput,
	TorqueTextInput,
	IncreaseStepButton,
	DecreaseStepButton,
	InvEnableCheckbox,

    // Events
	GoOnBus,
	ModeChange,
	SendMessages,
	TxPeriod,
	TorqueStepInc,
	TorqueStepDec,
	
    // Resources
    MainFont,
    TextFont
}

use AppId::*; // Shortcut
use std::thread;

nwg_template!(
    head: setup_ui<AppId>,
    controls: [
        (MainWindow, nwg_window!( title="CAN GUI"; size=(500, 240) )),
        (Label(0), nwg_label!( parent=MainWindow; text="CAN Bus:"; position=(5,15); size=(60, 25); font=Some(TextFont))),
		(Label(1), nwg_label!( parent=MainWindow; text="Torque Step:"; position=(5,80); size=(120, 25); font=Some(TextFont))),
		(Label(2), nwg_label!( parent=MainWindow; text="Torque Command:"; position=(5,120); size=(120,25); font=Some(TextFont))),
		(Label(3), nwg_label!( parent=MainWindow; text="Nm"; position=(215,80); size=(20,25); font=Some(TextFont))),
		(Label(4), nwg_label!( parent=MainWindow; text="Nm"; position=(215,120); size=(20,25); font=Some(TextFont))),
		(TimeKeeper, nwg_timer!(interval=1000)),
		(StartCANButton, nwg_button!(parent=MainWindow; text="Go On Bus"; position=(215, 8); size=(90, 30); font=Some(TextFont))),
		(Bus0Radio, nwg_radiobutton!(parent=MainWindow; text="Ch 0"; position=(80, 8); size=(50, 30); checkstate=CheckState::Checked; font=Some(TextFont))),
        (Bus1Radio, nwg_radiobutton!(parent=MainWindow; text="Ch 1"; position=(150, 8); size=(50, 30); font=Some(TextFont))),
		(ModeSelect, nwg_combobox!(parent=MainWindow; position=(360, 10); size=(130, 25); placeholder=Some("Mode Select"); font=Some(TextFont); collection=vec!["Battery","Powertrain"])),
		(SendMessagesButton, nwg_button!(parent=MainWindow; text="Start"; position=(5, 40); size=(90, 30); font=Some(TextFont); disabled=true)),
		(TorqueStepInput, nwg_textinput!(parent=MainWindow; text="0"; position=(130, 78); size=(80,20); font=Some(TextFont); visible=true)),
		(TorqueTextInput, nwg_textinput!(parent=MainWindow; text="0"; position=(130, 118); size=(80, 20); font=Some(TextFont); visible=true)),
		(IncreaseStepButton, nwg_button!(parent=MainWindow; text=">"; position=(260, 118); size=(20, 20); font=Some(TextFont); visible=true)),
		(DecreaseStepButton, nwg_button!(parent=MainWindow; text="<"; position=(240, 118); size=(20, 20); font=Some(TextFont); visible=true)),
		(InvEnableCheckbox, nwg_checkbox!(parent=MainWindow; text="Inverter Enable"; position=(360, 73); size=(110, 30); checkstate=CheckState::Unchecked; font=Some(TextFont); visible=true))
    ];
    events: [
		(StartCANButton, GoOnBus, Event::Click, |ui,_,_,_| {
			
			let bus_0_handle = nwg_get!(ui; (Bus0Radio, nwg::RadioButton));
			let bus_1_handle = nwg_get!(ui; (Bus1Radio, nwg::RadioButton));
			let bus_id: u8 = 2;
			
			if bus_0_handle.get_checkstate() == nwg::constants::CheckState::Checked {
				simple_message("Going on Bus", &format!("Connecting to Bus: {}", bus_0_handle.get_text()));
				let bus_id: u8 = 0;
			} else if bus_1_handle.get_checkstate() == nwg::constants::CheckState::Checked {
				simple_message("Going on Bus", &format!("Connecting to Bus: {}", bus_1_handle.get_text()));
				let bus_id: u8 = 1;
			} else {
				simple_message("Error", "No Bus Selected");
				return
			}
			
			unsafe {
				hndl = canOpenChannel(1, 0);
				if hndl < ERROR_OK {
					println!("Failed to open CAN channel. Error: {}", hndl);
					return
				}
			}
	
			let bitrate = -2; // 500 Kb/sec
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
			
			let start_enable = nwg_get!(ui; (SendMessagesButton, nwg::Button));
			start_enable.set_enabled(true);
		}),
		
		// Handle transmission of CAN Traffic
		// Some string type mismatch occurs during runtime // probably &str vs String (spoiler alert: that was it, fixed now)
		// Insanely inefficent -- GUI hangs for duration of CAN Timeout (go figure)
		(TimeKeeper, TxPeriod, Event::Tick, |ui,_,_,_| {
			println!("Sending Messages");
				let mode_handle = nwg_get!(ui; (ModeSelect, nwg::ComboBox<&str>));
				if mode_handle.get_selected_text() == "Battery" {
					// Send RTR messages
					let mut cell_voltage_rtr: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
					for id in 0x30f..0x330 {
						let result = unsafe {canWriteWait(hndl, id, cell_voltage_rtr.as_mut_ptr() as *mut c_void, 8, 1, 1000)};
						if result != ERROR_OK {
							println!("Failed to request cell voltage. Error: {}", result);
							return
						}
					}
				} else if mode_handle.get_selected_text() == "Powertrain" {
					// Read inputs from the Powertrain UI elements
				
					// Send Command Message
				
					// ID = 0xC0 (Hardcode)
					// Byte 0 and 1 = Torque Command
					// 		Adjustable Text Input Step Size + PageUp/PageDown Button Adjustment + Adjustable Text Input Torque Value
					// Byte 2 and 3 = Speed Command (Unused)
					// Byte 4 = Direction (Binary Toggle, default 0)
					// Byte 5 // Bit 0 = Enable (Button Toggle) // Bit 1 = Discharge (Always 0) // Bit 2 = Speed Mode Enable (Always 0)
					// Byte 6 and 7 = Torque Limit Command
					//		Adjustable Text Input
				
					let inverter_checkbox = nwg_get!(ui; (InvEnableCheckbox, nwg::CheckBox));
					let torque = nwg_get!(ui; (TorqueTextInput, nwg::TextInput));
				
					let id = 0x0C0;
				
					let mut inverter_enable = 0;
					if inverter_checkbox.get_checkstate() == nwg::constants::CheckState::Checked {
						inverter_enable = 1;
					}
				
					// Char1 = MyShort & 0xFF;
					// Char2 = MyShort >> 8;
				
					let torque_command: i16 = torque.get_text().parse().unwrap();
				
					let byte_five: u8 = inverter_enable;
					let byte_four: u8 = 0;
				
					// Torque Command
					let byte_zero: i16 = torque_command & 0xFF;
					let byte_one: i16 = torque_command >> 8;
				
					// Torque Limit Command
					let byte_six: u8 = 0;
					let byte_seven: u8 = 0;

					println!("Byte Zero: {}", byte_zero as u8);
					println!("Byte One: {}", byte_one as u8);
				
					let mut command_message_data: [u8; 8] = [byte_zero as u8, byte_one as u8, 0x00, 0x00, byte_four, byte_five, byte_six, byte_seven];
					let result = unsafe {canWriteWait(hndl, id, command_message_data.as_mut_ptr() as *mut c_void, 8, 1, 10000)};
					if result != ERROR_OK {
						println!("Torque Message Fucked Up. Error: {}", result);
						return
					}
				}
		}),
		
		
		// This event will handle the setup and display of the relevant UI elements
		// This event is currently fucking broken for no god damn reason what the fuck
		// 1/28/18 EVENT STILL DOESNT FIRE. PANIC. 
		(ModeSelect, ModeChange, Event::SelectionChanged, |ui,_,_,_| {
			println!("Mode Selection Changed");
			let mode_handle = nwg_get!(ui; (ModeSelect, nwg::ComboBox<String>));
			let start_enable = nwg_get!(ui; (SendMessagesButton, nwg::Button));
			start_enable.set_enabled(true);
			if mode_handle.get_selected_text() == "Battery" {
				//Battery
				
				//Do nothing, RTR is all that's needed

			} else if mode_handle.get_selected_text() == "Powertrain" {
				//Powertrain
				
				
				
				//Display Command Message parameters
				// ID = 0xC0 (Hardcode)
				// Byte 0 and 1 = Torque Command
				// 		Adjustable Text Input Step Size + PageUp/PageDown Button Adjustment + Adjustable Text Input Torque Value
				// Byte 2 and 3 = Speed Command (Unused)
				// Byte 4 = Direction (Binary Toggle)
				// Byte 5 // Bit 0 = Enable (Button Toggle) // Bit 1 = Discharge (Always 0) // Bit 2 = Speed Mode Enable (Always 0)
				// Byte 6 and 7 = Torque Limit Command
				//		Adjustable Text Input
			
			}
		
		}),
		
		// Start and Stop active session timer
		(SendMessagesButton, SendMessages, Event::Click, |ui,_,_,_| {
			let mut timer = nwg_get_mut!(ui; (TimeKeeper, nwg::Timer));
			let mut btn = nwg_get_mut!(ui; (SendMessagesButton, nwg::Button));
			
			if timer.running() {
				btn.set_text("Start");
				timer.stop();
			} else {
				btn.set_text("Stop");
				timer.start();
			}
		}),
		
		// Event handles editing of the Torque Command by Step Increase
		(IncreaseStepButton, TorqueStepInc, Event::Click, |ui,_,_,_| {
			let step = nwg_get_mut!(ui; (TorqueStepInput, nwg::TextInput));
			let torque = nwg_get_mut!(ui; (TorqueTextInput, nwg::TextInput));
			
			let step_value: i32 = step.get_text().parse().unwrap();
			let mut torque_value: i32 = torque.get_text().parse().unwrap();
			
			torque_value = torque_value + step_value;
			torque.set_text(&torque_value.to_string());
		}),
		
		// Event handles editing of the Torque Command by Step Decrease
		(DecreaseStepButton, TorqueStepDec, Event::Click, |ui,_,_,_| {
			let step = nwg_get_mut!(ui; (TorqueStepInput, nwg::TextInput));
			let torque = nwg_get_mut!(ui; (TorqueTextInput, nwg::TextInput));
			
			let step_value: i32 = step.get_text().parse().unwrap();
			let mut torque_value: i32 = torque.get_text().parse().unwrap();
			
			torque_value = torque_value - step_value;
			torque.set_text(&torque_value.to_string());
		})
		
    ];
    resources: [
        (MainFont, nwg_font!(family="Arial"; size=28)),
        (TextFont, nwg_font!(family="Arial"; size=16))
    ];
    values: []
);

fn main() {
	// CAN library initialization
	unsafe {canInitializeLibrary()};
	
	//instantiate UI with AppId shortcut
    let app: Ui<AppId>;

	//create UI and catch errors
    match Ui::new() {
        Ok(_app) => { app = _app; },
        Err(e) => { fatal_message("Fatal Error", &format!("{:?}", e) ); }
    }

	//draw windows and catch errors
    if let Err(e) = setup_ui(&app) {
        fatal_message("Fatal Error", &format!("{:?}", e));
    }

	//run event based UI
    dispatch_events();
	
	// Flush queue to be safe
	unsafe{canFlushReceiveQueue(hndl)};
	
	// Close bus
	let result = unsafe {canClose(hndl)};
		if result != ERROR_OK {
		println!("Failed to close bus. Error: {}", result);
		return
	}
}