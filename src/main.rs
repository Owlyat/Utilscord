use open_dmx::DMXSerial;
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
        }

        ratatui::restore();
    }

    fn handle_osc(&mut self) {
        match &mut self.tab_manager.osc_receiver {
            Some(rcvr) => match rcvr.recv_timeout(Duration::from_millis(50)) {
                Ok(packet) => match packet {
                    rosc::OscPacket::Message(osc_message) => {
                        self.tab_manager.osc_message_interaction(osc_message);
                    }
                    rosc::OscPacket::Bundle(osc_bundle) => {
                        self.tab_manager.osc_bundle_interaction(osc_bundle);
                    }
                },
                Err(_e) => {}
            },
            None => {}
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
                                Content::OSC(..) => {
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
                                Content::DMX(..) => match key.code {
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
    let mut utilschord = Utilscord::default();
    utilschord.run();
}
