use open_dmx::{DMXSerial, DMX_CHANNELS};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use std::env;
use std::time::Duration;

#[path = "applib/interact.rs"]
mod interact_mod;
use interact_mod::{component::Content, TabManager};
//mod ratatui_elements;
struct Utilscord {
    should_quit: bool,
    quit_key_code: Vec<KeyCode>,
    move_key_code: Vec<KeyCode>,
    _interact_key_code: KeyCode,
    tab_manager: TabManager,
}

impl Default for Utilscord {
    fn default() -> Self {
        Self {
            should_quit: false,
            quit_key_code: vec![KeyCode::Char('q'), KeyCode::Char('Q'), KeyCode::Esc],
            move_key_code: vec![KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down],
            _interact_key_code: KeyCode::Enter,
            tab_manager: TabManager::default(),
        }
    }
}

impl Utilscord {
    pub fn run(&mut self) {
        let mut terminal = ratatui::init();
        self.open_dmx();
        while !self.should_quit {
            terminal
                .draw(|frame: &mut Frame| {
                    frame.render_stateful_widget(
                        self.tab_manager.tabs[self.tab_manager.selected_tab].clone(),
                        frame.area(),
                        &mut self.tab_manager.tabs[self.tab_manager.selected_tab].content,
                    );
                })
                .unwrap();
            self.handle_events();
            self.handle_osc();
            self.check_dmx_state();
        }

        ratatui::restore();
    }
    fn open_dmx(&mut self) {
        if let Content::Dmx(_dimmer, _r, _g, _b, _adr, serial, dmx_status) =
            &mut self.tab_manager.tabs[2].content
        {
            match DMXSerial::open(if serial.is_empty() {
                match env::consts::OS {
                    "windows" => {
                        *serial = "COM3".into();
                        "COM3"
                    }
                    "linux" => {
                        *serial = "/dev/ttyUSB0".into();
                        "/dev/ttyUSB0"
                    }
                    _ => "",
                }
            } else {
                serial.trim()
            }) {
                Ok(dmx_chan) => {
                    self.tab_manager.dmx = Some(dmx_chan);
                }
                Err(e) => {
                    *dmx_status = format!(
                        "Error while opening default {} serial : {}",
                        if serial == "COM3" { "Windows" } else { "Linux" },
                        e
                    )
                }
            }
        }
    }
    fn check_dmx_state(&mut self) {
        if let Content::Dmx(dimmer, r, g, b, adr, serial, dmx_status) =
            &mut self.tab_manager.tabs[self.tab_manager.selected_tab].content
        {
            if let Some(dmx) = &mut self.tab_manager.dmx {
                let mut dmxs = [dimmer, r, g, b];
                dmxs.iter_mut().enumerate().for_each(|(id, chan)| {
                    let dmx_chan_adr = adr.wrapping_add(id).clamp(1, DMX_CHANNELS);
                    if !chan.clone().title.ends_with(&dmx_chan_adr.to_string()) {
                        let mut title = String::new();
                        for char in chan.title.chars() {
                            if !char.is_ascii_digit() {
                                title.push(char);
                            }
                        }
                        chan.title = format!("Fader : {}", dmx_chan_adr);
                    }
                });

                match dmx.check_agent() {
                    Ok(_) => {
                        *dmx_status = "Running".to_string();
                    }
                    Err(e) => {
                        if let Content::Dmx(dimmer, r, g, b, adr, _serial, dmx_status) =
                            &mut self.tab_manager.tabs[self.tab_manager.selected_tab].content
                        {
                            *dmx_status = format!("{}", e);
                            match DMXSerial::reopen(dmx) {
                                Ok(_) => {
                                    let mut dmxs = [dimmer, r, g, b];
                                    dmxs.iter_mut().enumerate().for_each(|(id, chan)| {
                                        let dmx_chan_adr =
                                            adr.wrapping_add(id).clamp(1, DMX_CHANNELS);
                                        if !chan.clone().title.ends_with(&dmx_chan_adr.to_string())
                                        {
                                            let mut title = String::new();
                                            for char in chan.title.chars() {
                                                if !char.is_ascii_digit() {
                                                    title.push(char);
                                                }
                                            }
                                            chan.title = format!("Fader : {}", dmx_chan_adr);
                                        }
                                    });
                                    *dmx_status = "Running".to_string();
                                }
                                Err(e) => {
                                    *dmx_status = format!("{}", e);
                                }
                            }
                        }
                    }
                }
            } else if !matches!(serial.as_str(), "COM3" | "/dev/ttyUSB0") {
                match DMXSerial::open(serial) {
                    Ok(dmx) => {
                        self.tab_manager.dmx = Some(dmx);
                    }
                    Err(e) => {
                        *dmx_status = format!("Error while opening from Serial {} : {}", serial, e);
                    }
                }
            } else {
                match DMXSerial::open(serial) {
                    Ok(dmx) => {
                        self.tab_manager.dmx = Some(dmx);
                    }
                    Err(e) => {
                        *dmx_status = format!(
                            "Error while opening from default {} Serial : {}",
                            if serial == "COM3" { "Windows" } else { "Linux" },
                            e
                        );
                    }
                }
            }
        }
    }

    fn handle_osc(&mut self) {
        if let Some(rcvr) = &mut self.tab_manager.osc_receiver {
            match rcvr.recv_timeout(Duration::from_millis(50)) {
                Ok(packet) => match packet {
                    rosc::OscPacket::Message(osc_message) => {
                        if let Ok(()) = self.tab_manager.osc_message_interaction(osc_message) {};
                    }
                    rosc::OscPacket::Bundle(osc_bundle) => {
                        self.tab_manager.osc_bundle_interaction(osc_bundle);
                    }
                },
                Err(_e) => {}
            }
        }
    }

    fn handle_events(&mut self) {
        let timeout = Duration::from_millis(50);

        if !event::poll(timeout).unwrap() {
            return;
        }
        if let Ok(event) = event::read() {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    // QUIT EVENT
                    for keycode in self.quit_key_code.clone() {
                        if (key.code, key.modifiers) == (keycode, KeyModifiers::CONTROL) {
                            self.should_quit = true;
                        }
                    }
                    // MOVE EVENT
                    for move_key_code in &mut self.move_key_code {
                        if key.code == *move_key_code {
                            match &self.tab_manager.get_selected_tab().content {
                                Content::MainMenu(..) => {
                                    if key.modifiers == KeyModifiers::SHIFT {
                                        match key.code {
                                            KeyCode::Up => {
                                                if !self.tab_manager.tabs[0].is_used() {
                                                    self.tab_manager.tabs[0]
                                                        .previous_content_element();
                                                }
                                            }
                                            KeyCode::Down => {
                                                if !self.tab_manager.tabs[0].is_used() {
                                                    self.tab_manager.tabs[0].next_content_element();
                                                }
                                            }
                                            KeyCode::Left => {
                                                if !self.tab_manager.tabs[0].is_used() {
                                                    self.tab_manager.previous()
                                                }
                                            }
                                            KeyCode::Right => {
                                                if !self.tab_manager.tabs[0].is_used() {
                                                    self.tab_manager.next()
                                                }
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                                Content::Osc(..) => {
                                    if key.modifiers == KeyModifiers::SHIFT {
                                        match key.code {
                                            KeyCode::Up => {
                                                if !self.tab_manager.tabs[1].is_used() {
                                                    self.tab_manager.tabs[1]
                                                        .previous_content_element()
                                                }
                                            }
                                            KeyCode::Down => {
                                                if !self.tab_manager.tabs[1].is_used() {
                                                    self.tab_manager.tabs[1].next_content_element()
                                                }
                                            }
                                            KeyCode::Left => self.tab_manager.previous(),
                                            KeyCode::Right => self.tab_manager.next(),
                                            _ => (),
                                        }
                                    }
                                }
                                Content::Dmx(..) => match key.code {
                                    KeyCode::Right => {
                                        if key.modifiers == KeyModifiers::SHIFT {
                                            self.tab_manager.next()
                                        } else {
                                            self.tab_manager.tabs[2].next_content_element()
                                        }
                                    }
                                    KeyCode::Left => {
                                        if key.modifiers == KeyModifiers::SHIFT {
                                            self.tab_manager.previous()
                                        } else {
                                            self.tab_manager.tabs[2].previous_content_element()
                                        }
                                    }
                                    _ => (),
                                },
                            }
                        }
                    }
                }
            }
            self.tab_manager.handle_event(event);
        }
    }
}

fn main() {
    Utilscord::default().run();
}
