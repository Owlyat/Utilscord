pub mod tab_mod;
use std::env;
use std::sync::mpsc::{self, Receiver, Sender};
use std::fs;
use std::time::{Duration, Instant};
use lofty::file::AudioFile;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use tab_mod::{Content, Input, SoundList, Tab, TabName};
use std::thread;
pub struct AppTab {
    pub tab : Vec<Tab>,
    pub selected_tab: usize,
    pub sender : Option<Sender<tab_mod::MusicState>>,
    pub receiver : Option<Receiver<f32>>,
}

impl AppTab {
    fn new() -> Self {
        let args: Vec<_> = env::args().collect();
        let mut app = AppTab {
            tab: vec![
                Tab {
                    tab_name: TabName::MainMenu, 
                    index: 0, 
                    content: Content::MainMenu(
                        SoundList::From(if args.len() > 1 {args[1].clone()} else {String::new()}),
                        Input {FieldTitle: "Path".to_owned(), selected : true,input : if args.len() > 1 {args[1].clone()} else {String::new()} ,..Default::default()}
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
        };
        if args.len() > 1 {app.tab[0].next();}
        app
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
                    input_field_logic(inputfield, key, keymod, soundlist);
                }
                // Selected Soundlist no fade edit
                else if soundlist.selected && !soundlist.editingfades{
                    if soundlist.state.selected() != None { 
                        match key {
                            
                            KeyCode::Up => {
                                if keymod == KeyModifiers::SHIFT {
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
                                if keymod == KeyModifiers::SHIFT {
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
                                let volume_changer_channel = mts.clone();
                                let volume_changer_channel_2 = mts.clone();
                                self.sender = Some(mts);
                                self.receiver = Some(mtr);
                                let fadein = soundlist.mp3_files[soundlist.state.selected().unwrap()].fade_tab_content[0].input.as_mut_str();
                                let fadein = if let x = fadein.trim().parse::<f32>().unwrap_or(0.0) {x} else {0.0};
                                let fadeout = soundlist.mp3_files[soundlist.state.selected().unwrap()].fade_tab_content[1].input.as_mut_str();
                                let fadeout = if let x = fadeout.trim().parse::<f32>().unwrap_or(0.0) {x} else {0.0};
                                let fade_in_duration = Duration::from_secs(fadein as u64);
                                let fade_out_duration = Duration::from_secs(fadeout as u64);
                                let song_duration = lofty::read_from_path(&std::path::Path::new(format!("{}/{}", soundlist.current_dir, soundlist.mp3_files[soundlist.state.selected().unwrap()].name).as_mut_str())).unwrap().properties().duration();
                                if fadein != 0.0 {
                                    let end_volume = soundlist.volume;
                                    let start_volume = soundlist.volume - soundlist.volume;

                                    // Spawn a new thread for the fading process
                                    thread::spawn(move || {
                                        let now = Instant::now();
                                
                                        loop {
                                            let volume = interpolate_value(now.elapsed(), fade_in_duration, start_volume, end_volume);
                                
                                            if volume_changer_channel.send(tab_mod::MusicState::VolumeChanged(volume)).is_err() {
                                                break; // Exit if the receiver is disconnected
                                            }
                                            if now.elapsed() >= fade_in_duration {
                                                break;
                                            }
                                            // Sleep briefly to avoid high CPU usage and control the update rate
                                            thread::sleep(Duration::from_millis(50));
                                        }
                                    });

                                }
                                if fadeout != 0.0 {
                                    let end_volume = soundlist.volume;
                                    let start_volume = soundlist.volume - soundlist.volume;

                                    thread::spawn(move || {
                                        let fade_out_start_point = song_duration - fade_out_duration;
                                        let now = Instant::now();
                                        loop {
                                            if now.elapsed() >= fade_out_start_point {

                                                let volume = interpolate_value(
                                                    song_duration.abs_diff(now.elapsed()), 
                                                    fade_out_duration, 
                                                    start_volume, 
                                                    end_volume
                                                );
                                                
                                                if volume_changer_channel_2.send(tab_mod::MusicState::VolumeChanged(volume)).is_err() {
                                                    break; // Exit if the receiver is disconnected
                                                }
                                            }
                                            
                                            
                                            if now.elapsed() >= song_duration {
                                                break;
                                            }
                                            thread::sleep(Duration::from_millis(50));
                                        }
                                    });
                                }

                                
                                
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
                // EDITING FADES
                } else if soundlist.editingfades {
                    fade_tab(soundlist, key, keymod);
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

fn input_field_logic(inputfield : &mut Input, key: KeyCode, keymod : KeyModifiers, soundlist : &mut SoundList) {
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
            inputfield.toggle();
            soundlist.current_dir = inputfield.input.clone();
            soundlist.update();
        },
        KeyCode::Backspace => {
            if keymod == KeyModifiers::CONTROL {
                inputfield.input.clear();
                inputfield.reset_cursor();
                return;
            } else {
                inputfield.delete_char()
            }
        },
        KeyCode::Left => inputfield.move_cursor_left(),
        KeyCode::Right => inputfield.move_cursor_right(),
        KeyCode::Esc => inputfield.toggle(),
        KeyCode::Char(to_insert) => inputfield.enter_char(to_insert),
        _ => {return;}
}}
}

fn fade_tab(soundlist : &mut SoundList, key: KeyCode, keymod : KeyModifiers) {
    let si = &mut soundlist.mp3_files[soundlist.state.selected().unwrap()];
    
    if si.fade_tab_content[0].input_mode {
        // Editing Fade In Input
        match key {
            KeyCode::Backspace => {
                if keymod == KeyModifiers::CONTROL {
                    si.fade_tab_content[0].input.clear();
                    si.fade_tab_content[0].reset_cursor();
                } else {
                    si.fade_tab_content[0].delete_char();
                }
            }
            KeyCode::Char(char_to_insert) => {
                match char_to_insert {
                    '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' | '.' => {
                        si.fade_tab_content[0].enter_char(char_to_insert);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
        

    if si.fade_tab_content[1].input_mode {
        // Editing Fade In Input
        match key {
            KeyCode::Backspace => {
                if keymod == KeyModifiers::CONTROL {
                    si.fade_tab_content[1].input.clear();
                    si.fade_tab_content[1].reset_cursor();
                } else {
                    si.fade_tab_content[1].delete_char();
                }
            }
            KeyCode::Char(char_to_insert) => {
                match char_to_insert {
                    '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' | '.' => {
                        si.fade_tab_content[1].enter_char(char_to_insert);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Navigating between Fade Inputs
    match key {
        KeyCode::Backspace => {soundlist.toggle_fade_edition();}
        KeyCode::Up => {si.previous_fade_tab();},
        KeyCode::Down => {si.next_fade_tab();},
        KeyCode::Left => {},
        KeyCode::Right => {},
        KeyCode::Enter => {si.edit();}
        KeyCode::Char('f') => {soundlist.toggle_fade_edition();},
        KeyCode::Esc => {soundlist.toggle_fade_edition();},
        _ => {}
    }
}

fn interpolate_value(elapsed: Duration, fade: Duration, start: f32, end: f32) -> f32 {
    let t = elapsed.as_secs_f32();

    if t <= fade.as_secs_f32() {
        // Interpolate from start to end during fade
        start + (end - start) * (t / fade.as_secs_f32()).clamp(0.0, 1.0)
    } else {
        end
    }
}