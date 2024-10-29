pub mod tab_mod;
use std::sync::mpsc::{self, Receiver, Sender};
use std::fs;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use tab_mod::{Content, Input, SoundList, Tab, TabName};
pub struct AppTab {
    pub tab : Vec<Tab>,
    pub selected_tab: usize,
    pub sender : Option<Sender<tab_mod::MusicState>>,
    pub receiver : Option<Receiver<f32>>,
}

impl AppTab {
    fn new() -> Self {
        
        AppTab {
            tab: vec![
                Tab {
                    tab_name: TabName::MainMenu, 
                    index: 0, 
                    content: Content::MainMenu(
                        SoundList::From(String::new()),
                        Input {FieldTitle: "Path".to_owned(), selected : true,..Default::default()}
                    )},
                Tab {
                    tab_name: TabName::Messages, 
                    index: 1, 
                    content: Content::None,
                }

            ],
            selected_tab : 0,
            sender : None,
            receiver : None,
        }
    }

    pub fn next(&mut self) {
        if self.tab.len()-1 != self.selected_tab {self.selected_tab += 1}
    }

    pub fn previous(&mut self) {
        if 0 != self.selected_tab {self.selected_tab -= 1}
    }

    pub fn content_next(&mut self) {
        if let Content::MainMenu(x,y ) = &mut self.tab[self.selected_tab].content {
            if y.input_mode == true {
                return;
            }
            if x.state.selected() != None {
                return;
            }
        }
        self.tab[self.selected_tab].next();
    }

    pub fn content_previous(&mut self) {
        if let Content::MainMenu(x,y ) = &mut self.tab[self.selected_tab].content {
            if y.input_mode == true {
                return;
            }
            if x.state.selected() != None {
                return;
            }
        }
        self.tab[self.selected_tab].previous();
    }

    pub fn interact(&mut self, key : KeyCode, keymod : KeyModifiers) {

        if let TabName::MainMenu = self.tab[self.selected_tab].tab_name {
            if let Content::MainMenu(soundlist,inputfield ) = &mut self.tab[self.selected_tab].content {
            // Selected inputfield
                if inputfield.selected {
                // Normal Mode
                    if inputfield.input_mode == false && key == KeyCode::Enter {
                        inputfield.toggle();
                        return}
                    // Edit Mode
                    else if inputfield.input_mode == true {
                        match key {
                            KeyCode::Char('V') => {
                                if keymod == KeyModifiers::CONTROL {
                                    inputfield.paste();
                                } else {
                                    inputfield.enter_char('v');
                                }}
                            KeyCode::Enter => {
                                inputfield.submit_message();
                                soundlist.current_dir = inputfield.input.clone();
                                soundlist.update();
                            },
                            KeyCode::Backspace => inputfield.delete_char(),
                            KeyCode::Left => inputfield.move_cursor_left(),
                            KeyCode::Right => inputfield.move_cursor_right(),
                            KeyCode::Esc => inputfield.toggle(),
                            KeyCode::Char(to_insert) => inputfield.enter_char(to_insert),
                            _ => {return;}
                    }}
                }
                // Selected Soundlist 
                else if soundlist.selected {
                    if soundlist.state.selected() != None { 
                        match key {
                            
                            KeyCode::Up => {
                                if keymod == KeyModifiers::SHIFT && !soundlist.editingfades {
                                    // Modify local volume
                                    match soundlist.modify_local_volume(soundlist.state.selected().unwrap(), soundlist.get_local_volume_of_selected_item() + 0.01) {
                                        Ok(_) => {}
                                        Err(e) => {panic!("{}",e)}
                                    }
                                    // Check if the playing music is the file selected on the list
                                    if soundlist.currently_playing == soundlist.mp3_files[soundlist.state.selected().unwrap()].name {
                                        if let Some(sender) = &mut self.sender {
                                            // Send local volume
                                            match sender.send(tab_mod::MusicState::LocalVolumeChanged(
                                                soundlist.get_local_volume_of_selected_item()
                                            )) {_ => {}}
                                        }

                                    }
                                } else {soundlist.PreviousSong();}
                            }
                                    
                           
                            KeyCode::Down => {
                                if keymod == KeyModifiers::SHIFT && !soundlist.editingfades  {
                                    // Modify local volume
                                    match soundlist.modify_local_volume(soundlist.state.selected().unwrap(), (soundlist.get_local_volume_of_selected_item() - 0.01).clamp(-2.0, 2.0)) {
                                        Ok(_) => {}
                                        Err(e) => {panic!("{}",e)}
                                    }
                                    // Check if the playing music is the file selected on the list
                                    if soundlist.currently_playing == soundlist.mp3_files[soundlist.state.selected().unwrap()].name {
                                        if let Some(sender) = &mut self.sender {
                                            // Send local volume
                                            match sender.send(tab_mod::MusicState::LocalVolumeChanged(
                                                soundlist.get_local_volume_of_selected_item()
                                            )) {_ => {}}
                                        }

                                    }
                                } 
                                else {soundlist.NextSong();}},
                           
                            KeyCode::Enter => {
                                if let Some(sender) = &mut self.sender {
                                    match sender.send((tab_mod::MusicState::Remove)) {
                                        _ => {}
                                    };}
                                soundlist.currently_playing = "".to_owned();
                                let (mts, wtr) = mpsc::channel();
                                let (wts, mtr) = mpsc::channel();
                                self.sender = Some(mts);
                                self.receiver = Some(mtr);
                                soundlist.play(wtr , wts);
                            },
                           
                            KeyCode::Esc => {
                                if soundlist.editingfades {
                                    soundlist.editingfades = false;
                                }
                                soundlist.Unselect();},
                           
                            KeyCode::Backspace | KeyCode::Delete => {
                                soundlist.currently_playing = "".to_owned();
                                if let Some(x) = &mut self.sender {
                                    match x.send((tab_mod::MusicState::Remove)) {
                                        _=> {}
                                    };}
                            }
                            
                            KeyCode::Char(' ') => {if let Some(sender) = &mut self.sender {
                                match sender.send((tab_mod::MusicState::PlayResume)) {
                                    _ => {}
                                };}}
                            
                            KeyCode::Char('+') => {
                                soundlist.volume += 0.01;
                                soundlist.volume = soundlist.volume.clamp(0.0, 2.0);
                                if let Some(sender) = &mut self.sender {
                                    match sender.send(tab_mod::MusicState::VolumeChanged(soundlist.volume)) {
                                        _ => {}
                                    };}
                            }
                           
                            KeyCode::Char('-') => {
                                soundlist.volume -= 0.01;
                                soundlist.volume = soundlist.volume.clamp(0.0, 2.0);
                                if let Some(sender) = &mut self.sender {
                                    match sender.send(tab_mod::MusicState::VolumeChanged(soundlist.volume)) {
                                        _ => {}
                                    };}}

                            KeyCode::Char('f') => {
                                soundlist.toggle_fade_edition();
                            }
                            _ => {}
                        }
                    }
                    else if soundlist.state.selected() == None && fs::read_dir(soundlist.current_dir.clone()).is_ok() {
                        match key {
                            KeyCode::Enter => {soundlist.PromptSelection()},
                            _ => {}
                        }
                    }
                }
            }}

            if let TabName::Messages = self.tab[self.selected_tab].tab_name {
            
            }
        }
}

impl Default for AppTab {
    fn default() -> Self {
        AppTab::new()
    }
}
