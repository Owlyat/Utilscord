#[path = "render.rs"]
mod render;
use clipboard;
use clipboard::windows_clipboard::WindowsClipboardContext;
use clipboard::ClipboardProvider;
use lofty::file::AudioFile;
use ratatui::prelude::*;
use ratatui::widgets::*;
use rodio::{Decoder, OutputStream, Sink};
use rosc::OscPacket;
use std::fs::{self, File};
use std::io::BufReader;
use std::net::{SocketAddrV4, UdpSocket};
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Tab {
    pub content: Content,
}

#[derive(Debug)]
pub struct Input {
    /// Current value of the input box
    pub input: String,
    /// Position of cursor in the editor area.
    pub character_index: usize,
    /// Current input mode
    pub input_mode: bool,
    // Input Title
    pub input_field_title: String,
    // Is selected
    pub is_selected: bool,
    pub edited: bool,
}

impl Tab {
    pub fn is_used(&self) -> bool {
        match self.content.clone() {
            Content::MainMenu(sound_list, input) => {
                if sound_list.editingfades
                    || sound_list.state.selected() != None
                    || input.input_mode
                {
                    true
                } else {
                    false
                }
            }
            Content::OSC(listening_ip_input, remote_ip_input) => {
                if listening_ip_input.edit_mode || remote_ip_input.edit_mode {
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn next_content_element(&mut self) {
        match &mut self.content {
            Content::MainMenu(sound_list, input) => {
                input.is_selected = !input.is_selected;
                sound_list.selected = !sound_list.selected;
            }
            Content::OSC(listening_ip_input, remote_ip_input) => {
                listening_ip_input.focus = !listening_ip_input.focus;
                remote_ip_input.focus = !remote_ip_input.focus;
                if listening_ip_input.focus == remote_ip_input.focus {
                    remote_ip_input.focus = !remote_ip_input.focus
                }
            }
        }
    }

    pub fn previous_content_element(&mut self) {
        match &mut self.content {
            Content::MainMenu(sound_list, input) => {
                input.is_selected = !input.is_selected;
                sound_list.selected = !sound_list.selected;
            }
            Content::OSC(listening_ip_input, remote_ip_input) => {
                listening_ip_input.focus = !listening_ip_input.focus;
                remote_ip_input.focus = !remote_ip_input.focus;
                if listening_ip_input.focus == remote_ip_input.focus {
                    remote_ip_input.focus = !remote_ip_input.focus
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Content {
    MainMenu(SoundList, Input),
    OSC(IPInput, IPInput),
}

impl Content {
    pub fn to_string(&self) -> String {
        match self {
            Content::MainMenu(_sound_list, _input) => "Main Menu".to_owned(),
            Content::OSC(_listening_ip_input, _remote_ip_input) => "OSC".to_owned(),
        }
    }
}

impl Input {
    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    pub fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    pub fn paste(&mut self) {
        let mut clip = WindowsClipboardContext::new().unwrap();
        match clip.get_contents() {
            Ok(text) => {
                for char in text.chars() {
                    self.enter_char(char);
                }
            }
            Err(_e) => {}
        }
    }

    pub fn toggle(&mut self) {
        self.input_mode = !self.input_mode;
    }
}

impl Clone for Input {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            character_index: self.character_index.clone(),
            input_mode: self.input_mode.clone(),
            input_field_title: self.input_field_title.clone(),
            is_selected: self.is_selected.clone(),
            edited: self.edited.clone(),
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Input {
            input: String::new(),
            character_index: 0,
            input_mode: false,
            input_field_title: String::new(),
            is_selected: false,
            edited: false,
        }
    }
}

//IPINPUT
#[derive(Debug, Clone)]
pub struct IPInput {
    pub input: String,
    pub focus: bool,
    pub edit_mode: bool,
    pub title: String,
    character_index: usize,
    pub _osc_receiver: Option<OscPacket>,
}

impl IPInput {
    //IPINPUT FUNCTIONNALITIES
    pub fn new(title: String) -> Self {
        IPInput {
            title,
            focus: false,
            input: String::new(),
            edit_mode: false,
            character_index: 0,
            _osc_receiver: None,
        }
    }

    pub fn toggle_edit_mode(&mut self) -> Result<Receiver<OscPacket>, ()> {
        if self.edit_mode {
            self.edit_mode = false;
            match self.submit_message() {
                Ok(rcv) => return Ok(rcv),
                Err(()) => return Err(()),
            };
        } else {
            self.edit_mode = true;
            Err(())
        }
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) -> Result<Receiver<OscPacket>, ()> {
        self.reset_cursor();
        let _default = SocketAddrV4::from_str("127.0.0.1:8000");
        let addr = SocketAddrV4::from_str(&self.input);

        match addr {
            Ok(v) => match UdpSocket::bind(v) {
                Ok(sock) => {
                    let (sender, receiver) = mpsc::channel();

                    thread::spawn(move || {
                        let mut buf = [0u8; rosc::decoder::MTU];
                        loop {
                            match sock.recv_from(&mut buf) {
                                Ok((size, _addr)) => {
                                    match rosc::decoder::decode_udp(&buf[..size]) {
                                        Ok((_, packet)) => match sender.send(packet) {
                                            Ok(_) => {}
                                            Err(_) => {}
                                        },
                                        Err(_e) => {}
                                    };
                                }
                                Err(_e) => {}
                            }
                        }
                    });
                    Ok(receiver)
                }
                Err(e) => {
                    self.input = format!("{}", e);
                    Err(())
                }
            },
            Err(e) => {
                self.input = format!("{}", e);
                Err(())
            }
        }
    }
}

impl Clone for Content {
    fn clone(&self) -> Content {
        match self {
            Content::MainMenu(soundlist, input) => {
                return Content::MainMenu(soundlist.clone(), input.clone())
            }
            Content::OSC(listening_ip_input, remote_ip_input) => {
                return Content::OSC(listening_ip_input.clone(), remote_ip_input.clone())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoundList {
    // List of Sound Files
    pub sound_files: Vec<SoundItem>,
    pub state: ListState,
    pub current_dir: String, // Store the current directory path
    pub selected: bool,
    pub currently_playing: String,
    pub volume: f32, // General Volume
    pub editingfades: bool,
}

#[derive(Clone, Debug)]
pub struct SoundItem {
    pub name: String,
    pub selected: bool,
    pub local_volume: f32, // Local Volume
    selected_fade_tab: usize,
    pub fade_tab_content: Vec<Input>,
}

impl SoundItem {
    pub fn next_fade_tab(&mut self) {
        if self.selected_fade_tab != 1 {
            self.selected_fade_tab += 1;
        }
        for i in &mut self.fade_tab_content {
            i.is_selected = false
        }
        self.fade_tab_content[self.selected_fade_tab].is_selected = true;
    }

    pub fn previous_fade_tab(&mut self) {
        if self.selected_fade_tab != 0 {
            self.selected_fade_tab -= 1
        };
        for i in &mut self.fade_tab_content {
            i.is_selected = false
        }
        self.fade_tab_content[self.selected_fade_tab].is_selected = true;
    }

    pub fn edit(&mut self) {
        self.fade_tab_content[self.selected_fade_tab].input_mode =
            !self.fade_tab_content[self.selected_fade_tab].input_mode
    }
}

pub enum MusicState {
    PlayResume,
    Remove,
    VolumeChanged(f32),
    LocalVolumeChanged(f32),
}

impl SoundList {
    pub fn from_dir(dir: String) -> Self {
        let sound_files = SoundList::get_sound_files_from_dir(dir.clone());
        Self {
            sound_files,
            state: ListState::default(),
            current_dir: dir.clone(),
            selected: false,
            currently_playing: String::new(),
            volume: 1.0,
            editingfades: false,
        }
    }

    pub fn toggle_fade_edition(&mut self) {
        if self.sound_files[self.state.selected().unwrap()].fade_tab_content[0].input_mode {
            return;
        }
        if self.sound_files[self.state.selected().unwrap()].fade_tab_content[1].input_mode {
            return;
        }
        self.editingfades = !self.editingfades
    }

    pub fn get_local_volume_of_selected_item(&self) -> f32 {
        self.sound_files[self.state.selected().unwrap()].local_volume
    }

    pub fn get_local_volume_of_item_index(&self, index: usize) -> f32 {
        self.sound_files[index].local_volume
    }

    pub fn modify_local_volume(&mut self, index: usize, new_volume: f32) -> Result<(), String> {
        if index <= self.sound_files.len() - 1 {
            self.sound_files[index].local_volume = new_volume.clamp(-2.0, 2.0);
            return Ok(());
        } else {
            return Err(format!(
                "Index : [{}] is out of bound \n    => Lenght : {}\n    {} <= {}",
                index,
                self.sound_files.len() - 1,
                index,
                self.sound_files.len() - 1
            ));
        };
    }

    pub fn play(&mut self, receiver: Receiver<MusicState>, sender: Sender<f32>, index: usize) {
        // local index
        // Offset Volume on each song
        let local_volume = self.sound_files[index].local_volume;
        let general_volume = self.volume;
        self.currently_playing = self.sound_files[index].name.clone();
        let arc_self = Arc::new(Mutex::new(self.clone()));
        thread::spawn(move || {
            let soundlist = arc_self.lock().unwrap();
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            let file = BufReader::new(
                File::open(format!(
                    "{}/{}",
                    soundlist.current_dir, soundlist.sound_files[index].name
                ))
                .unwrap(),
            );
            let source = Decoder::new(file).unwrap();
            sink.append(source);
            let mut gv: f32 = general_volume;
            let mut lv: f32 = local_volume;
            loop {
                if gv + lv <= 0.0 {
                    sink.set_volume(0.0);
                } else {
                    sink.set_volume(gv + lv);
                }
                for i in receiver.iter() {
                    match i {
                        MusicState::Remove => {
                            sink.clear();
                            match sender.send(sink.volume()) {
                                _ => {}
                            };
                            break;
                        }
                        MusicState::PlayResume => {
                            if sink.is_paused() {
                                sink.play();
                            } else {
                                sink.pause();
                            }
                        }
                        MusicState::VolumeChanged(new_volume) => {
                            // aply volume
                            gv = new_volume;
                            if gv + lv <= 0.0 {
                                sink.set_volume(0.0);
                            } else {
                                sink.set_volume(gv + lv);
                                match sender.send(sink.volume()) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                };
                            }
                        }
                        MusicState::LocalVolumeChanged(new_local_volume) => {
                            lv = new_local_volume;
                            if gv + lv <= 0.0 {
                                sink.set_volume(0.0);
                            } else {
                                sink.set_volume(gv + lv);
                                match sender.send(sink.volume()) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                };
                            }
                        }
                    }
                }
                if sink.empty() {
                    break;
                }
            }
            drop(receiver);
        });
    }

    fn get_list_items(&self) -> Vec<ListItem> {
        let file_duration = match self.state.selected() {
            Some(_v) => {
                let file_duration = lofty::read_from_path(&Path::new(
                    format!(
                        "{}/{}",
                        self.current_dir,
                        self.sound_files[self.state.selected().unwrap()].name
                    )
                    .as_str(),
                ))
                .unwrap()
                .properties()
                .duration();
                file_duration
            }
            None => Duration::from_secs(0),
        };
        self.sound_files
            .iter()
            .map(|si| {
                // Check if local volume is not edited
                return ListItem::new(
                    if let 0.0 = format!("{:.2}", si.local_volume)
                        .trim()
                        .parse::<f32>()
                        .unwrap()
                    {
                        Text::from(vec![
                            // Song Title
                            Line::from(vec![Span::styled(
                                si.name.clone(),
                                Style::default().fg(Color::White),
                            )])
                            .left_aligned()
                            .fg(Color::White),
                            // Fade Text
                            Line::from(vec![if let true = si.selected {
                                Span::styled("Press F to edit Fades", Style::default())
                            } else {
                                Span::styled("", Style::default().fg(Color::White))
                            }])
                            .right_aligned()
                            .fg(Color::Yellow),
                            if let true = si.selected {
                                Line::from(Span::styled(
                                    format!("{} secondes", file_duration.as_secs().to_string()),
                                    Style::default(),
                                ))
                            } else {
                                Line::from("")
                            },
                        ])
                    } else {
                        Text::from(vec![
                            // Song Title
                            Line::from(vec![Span::styled(
                                si.name.clone(),
                                Style::default().fg(Color::White),
                            )])
                            .left_aligned(),
                            // Local Volume
                            Line::from(Span::styled(
                                format!("Local Volume : {:.2}", si.local_volume),
                                Style::default().fg(Color::Yellow),
                            ))
                            .centered(),
                            // Fade Text
                            Line::from(vec![if let true = si.selected {
                                Span::styled(
                                    "Press F to edit Fades",
                                    Style::default().fg(Color::Yellow),
                                )
                            } else {
                                Span::styled("", Style::default().fg(Color::White))
                            }])
                            .right_aligned(),
                            Line::from(vec![if let true = si.selected {
                                Span::styled(
                                    format!("{} secondes", file_duration.as_secs().to_string()),
                                    Style::default(),
                                )
                            } else {
                                Span::styled("", Style::default())
                            }]),
                        ])
                    },
                );
            })
            .collect()
    }

    pub fn unselect(&mut self) {
        self.toggle_status();
        self.state.select(None);
        self.toggle_status();
    }

    pub fn next_song(&mut self) {
        self.toggle_status();
        self.state.select_next();
        self.toggle_status();
    }

    pub fn previous_song(&mut self) {
        self.toggle_status();
        if self.state.selected().unwrap() == 0 {
            self.state.select(Some(self.sound_files.len() - 1));
            self.toggle_status();
            return;
        }
        self.state.select_previous();
        self.toggle_status();
    }

    pub fn select_song(&mut self, index: usize) {
        self.toggle_status();
        self.state.select(Some(index));
        self.toggle_status();
    }

    pub fn toggle_status(&mut self) {
        if let Some(i) = self.state.selected() {
            if self.sound_files.len() - 1 >= i {
                self.sound_files[i].selected = !self.sound_files[i].selected
            } else {
                self.state.select_first();
                self.toggle_status();
            }
        }
    }

    pub fn prompt_selection(&mut self) {
        self.state.select_first();
        self.toggle_status();
    }

    // Function to get sound files from a folder
    fn get_sound_files_from_dir<P: AsRef<Path>>(folder_path: P) -> Vec<SoundItem> {
        let mut sound_files = Vec::new();
        if let Ok(entries) = fs::read_dir(folder_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension == "mp3" || extension == "wav" {
                            if let Some(file_name) = path.file_name() {
                                sound_files.push(SoundItem {
                                    name: file_name.to_string_lossy().into_owned(),
                                    selected: false,
                                    local_volume: 0.0,
                                    selected_fade_tab: 0,
                                    fade_tab_content: vec![
                                        Input {
                                            input_field_title: "Fade In Time".to_owned(),
                                            is_selected: true,
                                            ..Default::default()
                                        },
                                        Input {
                                            input_field_title: "Fade Out Time".to_owned(),
                                            ..Default::default()
                                        },
                                    ],
                                });
                            }
                        }
                    }
                }
            }
        }
        sound_files
    }

    pub fn update(&mut self) {
        self.sound_files = SoundList::get_sound_files_from_dir(self.current_dir.clone())
    }
}
