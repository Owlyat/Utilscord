use crate::interact_mod::component::{Content, DMXInput};
use open_dmx::{DMXSerial, DMX_CHANNELS};
use std::env;

#[derive(Debug)]
pub struct DMXHandler {
    pub dmx_connection_option: Option<DMXSerial>,
}

impl DMXHandler {
    fn empty() -> Self {
        Self {
            dmx_connection_option: None,
        }
    }
    /// Open a dmx connection. Returns a DMXHandler Holding the connection.
    /// This function is called once at the start of the application.
    pub fn open_dmx(ctx: &mut Content) -> Result<Self, Self> {
        if let Content::Dmx(_dimmer, _r, _g, _b, _adr, serial, dmx_status) = ctx {
            match DMXSerial::open(if serial.is_empty() {
                match env::consts::OS {
                    "windows" => {
                        *serial = "COM3".into();
                        serial
                    }
                    "linux" => {
                        *serial = "/dev/ttyUSB0".into();
                        serial
                    }
                    _ => "",
                }
            } else {
                serial
            }) {
                Ok(dmx_chan) => {
                    *dmx_status = "Running".into();
                    return Ok(Self {
                        dmx_connection_option: Some(dmx_chan),
                    });
                }
                Err(e) => {
                    *dmx_status = format!(
                        "Error while opening {} serial : {}",
                        if serial == "COM3" {
                            "Windows"
                        } else if serial == "/dev/ttyUSB0" {
                            "Linux"
                        } else {
                            ""
                        },
                        e
                    );
                    return Err(DMXHandler::empty());
                }
            }
        }
        panic!("Content Found is not DMX Content");
    }
    /// Main function to handle the dmx connection. Called every frame to update dmx status.
    pub fn handle_dmx(&mut self, ctx: &mut Content) -> Result<(), ()> {
        if let Some(dmx_connection) = &mut self.dmx_connection_option {
            if dmx_connection.check_agent().is_ok() {
                // DMX Connection exists
                self.handle_faders(ctx);
            } else {
                // DMX Connection does not exists
                self.reset_faders(ctx);
                return Err(());
            }
        } else if self.reconnect(ctx).is_err() {
            return Err(());
        }
        Ok(())
    }
    /// Reset the faders titles to String::new()
    fn reset_faders(&self, ctx: &mut Content) {
        if let Content::Dmx(f1, f2, f3, f4, ..) = ctx {
            let mut dmx_faders = [f1, f2, f3, f4];
            dmx_faders
                .iter_mut()
                .map(|dmx_fader| {
                    if !dmx_fader.title.is_empty() {
                        Some(dmx_fader)
                    } else {
                        None
                    }
                })
                .for_each(|fader| {
                    if let Some(fader) = fader {
                        fader.title = String::new()
                    }
                });
        }
    }
    /// Set the faders Title to the actual dmx adress incremented fader by fader;
    /// Example : if your adress is 124, fader 1 will be set to 124, fader 2 will be set to 125 , etc ...
    fn handle_faders(&mut self, ctx: &mut Content) {
        if let Content::Dmx(f1, f2, f3, f4, adr, _serial, dmx_status) = ctx {
            let mut dmx_faders: [&mut DMXInput; 4] = [f1, f2, f3, f4];
            dmx_faders.iter_mut().enumerate().for_each(|(id, fader)| {
                let current_adress = adr.wrapping_add(id).clamp(1, DMX_CHANNELS);
                if !&fader.title.ends_with(&current_adress.to_string()) {
                    let mut title = String::new();
                    fader.title.chars().for_each(|c| {
                        if !c.is_ascii_digit() {
                            title.push(c);
                        }
                    });
                    fader.title = format!("Fader : {}", current_adress);
                }
                *dmx_status = "Running".into();
            });
        }
    }
    /// Will try to reconnect to the dmx connection if found or will create a new one based on the serial
    fn reconnect(&mut self, ctx: &mut Content) -> Result<(), ()> {
        if let Content::Dmx(.., serial, dmx_status) = ctx {
            if let Some(dmx_conn) = &mut self.dmx_connection_option {
                // if this does not work we'll create a new dmx connection
                if dmx_conn.reopen().is_ok() {
                    return Ok(());
                }
            }
            // new connection created
            match DMXSerial::open(serial) {
                Ok(dmx_chan) => {
                    self.dmx_connection_option = Some(dmx_chan);
                }
                Err(e) => {
                    *dmx_status = format!(
                        "Error while opening {} serial : {}",
                        if serial == "COM3" {
                            "Windows"
                        } else if serial == "/dev/ttyUSB0" {
                            "Linux"
                        } else {
                            ""
                        },
                        e
                    );
                    return Err(());
                }
            }
        }

        Ok(())
    }
    /// Use this function after you changed the DMX Content, like fader value, dmx adress.
    pub fn update_dmx(&mut self, ctx: &mut Content) {
        if let Content::Dmx(f1, f2, f3, f4, adr, _serial, dmx_status) = ctx {
            if let Some(dmx_connection) = &mut self.dmx_connection_option {
                let mut dmxs = [f1, f2, f3, f4];
                dmxs.iter_mut().enumerate().for_each(|(id, dmx)| {
                    let dmx_channel_adress: usize = adr.wrapping_add(id).clamp(1, DMX_CHANNELS);
                    match open_dmx::check_valid_channel(dmx_channel_adress) {
                        Ok(_) => {
                            if dmx_connection
                                .set_channel(dmx_channel_adress, dmx.value)
                                .is_ok()
                            {
                                // DMX SUCESSFULY SET TO VALUE
                                *dmx_status =
                                    format!("set {} to {}", dmx_channel_adress, dmx.value);
                            }
                        }
                        Err(e) => *dmx_status = format!("{}", e),
                    }
                });
            }
        }
    }
}
