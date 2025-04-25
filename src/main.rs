use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui_explorer::FileExplorer;
use std::time::Duration;
use std::{path::Path, rc::Rc};

#[path = "applib/interact.rs"]
mod interact_mod;
use interact_mod::{component::Content, TabManager};
mod dmx;

struct Utilscord {
    should_quit: bool,
    quit_key_code: Rc<[KeyCode]>,
    tab_manager: TabManager,
}

impl Default for Utilscord {
    fn default() -> Self {
        Self {
            should_quit: false,
            quit_key_code: Rc::new([KeyCode::Char('q'), KeyCode::Char('Q'), KeyCode::Esc]),
            tab_manager: TabManager::default(),
        }
    }
}

impl Utilscord {
    pub fn run(&mut self) {
        let mut terminal = ratatui::init();
        let mut file_explorer = FileExplorer::new().unwrap();
        file_explorer
            .set_cwd(
                std::fs::canonicalize(Path::new("."))
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            )
            .unwrap();
        loop {
            if self.should_quit {
                break;
            }
            if terminal
                .draw(|f| self.tab_manager.draw(f, &mut file_explorer))
                .is_err()
            {
                break;
            }
            self.handle_events(&mut file_explorer);
            self.handle_osc();
            let _ = self
                .tab_manager
                .dmx_handler
                .handle_dmx(&mut self.tab_manager.tabs[self.tab_manager.selected_tab].content);
        }

        ratatui::restore();
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

    fn handle_events(&mut self, file_explorer: &mut ratatui_explorer::FileExplorer) {
        let timeout = Duration::from_millis(50);

        if !event::poll(timeout).unwrap() {
            return;
        }
        if let Ok(event) = event::read() {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    // QUIT EVENT
                    let keycodes = Rc::clone(&self.quit_key_code);
                    for keycode in keycodes.iter() {
                        if (key.code, key.modifiers) == (*keycode, KeyModifiers::CONTROL) {
                            self.should_quit = true;
                        }
                    }
                    // MOVE EVENT
                    match &self.tab_manager.get_selected_tab_mut().content {
                        Content::MainMenu(..) => {
                            if key.modifiers == KeyModifiers::SHIFT {
                                match key.code {
                                    KeyCode::Char('j' | 'J') => {
                                        if !self.tab_manager.tabs[0].is_used() {
                                            self.tab_manager.tabs[0].next_content_element();
                                        }
                                    }
                                    KeyCode::Char('k' | 'K') => {
                                        if !self.tab_manager.tabs[0].is_used() {
                                            self.tab_manager.tabs[0].previous_content_element();
                                        }
                                    }
                                    KeyCode::Up => {
                                        if !self.tab_manager.tabs[0].is_used() {
                                            self.tab_manager.tabs[0].previous_content_element();
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
                                            self.tab_manager.tabs[1].previous_content_element()
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
            self.tab_manager.handle_event(event, file_explorer);
        }
    }
}

fn main() {
    Utilscord::default().run();
}
