use block::Title;
use clipboard::windows_clipboard::WindowsClipboardContext;
use clipboard::ClipboardProvider;
use ratatui::prelude::*;
use ratatui::widgets::*;
use style::Stylize;
use rodio::{Decoder,OutputStream,Sink};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::{Arc,Mutex};
use std::io::BufReader;
use std::fs::{self, File};
use std::path::Path;
use std::thread;
use clipboard;

#[derive(Clone, Debug)]
pub enum TabName {
    MainMenu,
    Messages,
}

impl TabName {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub tab_name: TabName,
    pub index: usize,
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
    pub FieldTitle : String,
    pub selected : bool,
    pub edited : bool,
}

impl Tab {
    pub fn next(&mut self) {
        if let Content::MainMenu(x,y ) = &mut self.content {
            x.selected = !x.selected;
            y.selected = !y.selected;
        }
    }

    pub fn previous(&mut self) {
        if let Content::MainMenu(x,y ) = &mut self.content {
            x.selected = !x.selected;
            y.selected = !y.selected;
        }
    }
}

impl Input {
    pub const fn new() -> Self {
        Self {
            input: String::new(),
            character_index: 0,
            input_mode: false,
            FieldTitle: String::new(),
            selected: false,
            edited : false,
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

    /// Returns the byte index based on the character position.
    ///
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

    pub fn submit_message(&mut self) {
        self.toggle();
    }

    pub fn paste(&mut self) {
        let mut clip = WindowsClipboardContext::new().unwrap();
        match clip.get_contents() {
            Ok(text) => {
                for char in text.chars() {
                    self.enter_char(char);
                }
            },
            Err(e) => {}
        }
    }

    pub fn toggle(&mut self) {
        self.input_mode = !self.input_mode;
    }
}

impl Clone for Input {
    fn clone(&self) -> Self {
        Self {
            input : self.input.clone(),
            character_index: self.character_index.clone(),
            input_mode: self.input_mode.clone(),
            FieldTitle: self.FieldTitle.clone(),
            selected: self.selected.clone(),
            edited: self.edited.clone()
        }
    }
}

impl Default for Input {
    fn default() -> Self {

        Input {
            input: String::new(),
            character_index: 0,
            input_mode: false,
            FieldTitle: String::new(),
            selected : false,
            edited: false,
        }
    }
}

impl StatefulWidget for Input {
    type State = String;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {

        let input = Paragraph::new(match self.input_mode {
            false => {
                Line::from(self.input).fg(Color::White)
            },
            true => {Line::from(self.input).fg(Color::Yellow)}
        })
            .style(match self.input_mode {
                false => Style::default(),
                true => Style::default().yellow(),
            })
            .block(Block::bordered().title(
                Line::from(match self.input_mode {
                    false => {
                        Line::from(state.to_string())
                        .centered()
                        .fg(Color::White)
                    },
                    true => {
                        Line::from(format!("{} - Edit", state.to_string()))
                        .centered()
                        .fg(Color::Yellow)
                    }
                })
            ).fg(match self.selected {
                true => {Color::Yellow},
                false => {Color::White},
            }))
            .render(area, buf);
    }

}

#[derive(Debug)]
pub enum Content {
    MainMenu(SoundList, Input),
    None,
}

impl Clone for Content {
    fn clone(&self) -> Content {

        match self {
            Content::MainMenu(soundList, Input) => {
                return Content::MainMenu(SoundList::from(soundList.clone()), Input.clone())
            },
            Content::None => {
               return Content::None
            }
        }
    }
}

impl StatefulWidget for Tab {
    type State = TabName;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {

        let main_tab_border = Block::bordered()
            .title(
                Title::from(state.to_string())
                .position(block::Position::Top)
                .alignment(Alignment::Center)
            ).fg(Color::White)
            .title(
                Title::from("| Press <Shift> + ◄ ► to change Tab |")
                .position(block::Position::Bottom)
                .alignment(Alignment::Left)
            ).fg(Color::White)
            .title(
                Title::from("| Press <Shift> + ▲ ▼ To navigate |")
                .position(block::Position::Bottom)
                .alignment(Alignment::Right)
            )
            .style(Style::default()).white()
            .bg(Color::Black);
        let tab_content = main_tab_border.inner(area);
        main_tab_border.render(area, buf);

        match state {
            TabName::MainMenu => {
                let vert = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Fill(3),
                ]);

                let [ tab_content, tab_footer] = vert.areas(tab_content);

                match self.content {
                    Content::MainMenu(soundlist,Input ) => {

                        let mut copy = Input.clone();
                        Input.render(tab_content, buf, &mut copy.FieldTitle);

                        let mut copy = soundlist.clone();
                        soundlist.render(tab_footer, buf, &mut copy.state);
                    },
                    Content::None => {}
                }
            },
            TabName::Messages => {
                let vert = Layout::vertical([
                    Constraint::Fill(1),
                    Constraint::Fill(3),
                ]);
                let [ tab_content, _tab_footer] = vert.areas(tab_content);
                let items: Vec<ListItem> = vec![];
                let pagelist = List::new(items)
                    .block(block::Block::bordered())
                    .highlight_style(Style::default())
                    .white()
                    .highlight_symbol("➡️")
                    .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);
                StatefulWidget::render(pagelist, tab_content, buf, &mut ListState::default());
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoundList {
    pub mp3_files: Vec<SoundItem>, // Store the MP3 file names
    pub state: ListState,
    pub current_dir: String, // Store the current directory path
    pub selected : bool,
    pub currently_playing : String,
    pub volume : f32,
}

#[derive(Clone, Debug)]
struct SoundItem {
    name : String,
    selected : bool,
    local_volume : f32,
}

pub enum MusicState {
    PlayResume,
    Remove,
    VolumeUp(f32),
    VolumeDown(f32),
}

impl SoundList {
    pub fn From(dir: String) -> Self {

        let mp3_files = SoundList::get_mp3_files_from_dir(dir.clone());
        Self {
            mp3_files,
            state: ListState::default(),
            current_dir: dir.clone(),
            selected : false,
            currently_playing : String::new(),
            volume : 0.0,
        }
    }

    pub fn play(&mut self, Receiver: Receiver<MusicState>, sender : Sender<f32>) {

        // Offset Volume on each song
        let local_volume = self.mp3_files[self.state.selected().unwrap_or(0)].local_volume;

        self.currently_playing = self.mp3_files[self.state.selected().unwrap()].name.clone();
        let arc_self = Arc::new(Mutex::new(self.clone()));
        thread::spawn(move || {
            let mut soundlist = arc_self.lock().unwrap();
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink= Sink::try_new(&stream_handle).unwrap();
            let file = BufReader::new(File::open(format!("{}/{}",soundlist.current_dir, soundlist.mp3_files[soundlist.state.selected().unwrap()].name)).unwrap());
            let source = Decoder::new(file).unwrap();
            sink.append(source);
            loop {
                for i in Receiver.iter() {
                    match i {
                        MusicState::Remove => {sink.clear();sender.send((sink.volume()));},
                        MusicState::PlayResume => {
                            if sink.is_paused() {sink.play();}
                            else {sink.pause();}
                        }
                        MusicState::VolumeDown(x) => {
                            // No volume
                            if sink.volume() == 0.0 {
                                match sender.send((0.0)) {
                                    Ok(_) => {},
                                    Err(_) => {}
                                }; 
                                return;
                            };
                            
                            // aply volume
                            sink.set_volume(match format!("{:.1}",(sink.volume() - 0.1).abs()).parse::<f32>() {
                                Ok(f) => f,
                                Err(e) => {1.0},
                            } );
                            match sender.send((sink.volume())) {
                                Ok(_) => {},
                                Err(_) => {},
                            };
                        }
                        MusicState::VolumeUp(x) => {
                            // Max Volume
                            if sink.volume() == 2.0 {
                                match sender.send((sink.volume())) {
                                    Ok(_) => {},
                                    Err(_) => {},
                                }; 
                                return;
                            };

                            // aply volume
                            sink.set_volume(format!("{:.1}",(sink.volume() + 0.1).abs()).parse().unwrap());
                            match sender.send((sink.volume())) {
                                Ok(_) => {},
                                Err(_) => {}
                            };
                        }
                    }
                }
                if sink.empty() {
                    break;
                }
            }

        });

    }

    fn get_list_items(&self) -> Vec<ListItem> {
        self.mp3_files.iter()
        .map(|si| {
            if let Some(i) = self.state.selected() {
                return ListItem::new(si.name.clone()).fg(Color::White)
            }
            ListItem::new(si.name.clone())
        })
        .collect()
    }

    pub fn Unselect(&mut self) {
        self.state.select(None);
        self.toggle_status();
    }

    pub fn NextSong(&mut self) {
        self.state.select_next();
        self.toggle_status();
    }
    
    pub fn PreviousSong(&mut self) {
        if self.state.selected().unwrap() == 0 {
            self.state.select(Some(self.mp3_files.len() -1));
            self.toggle_status();
            return;
        }
        self.state.select_previous();
        self.toggle_status();
    }

    pub fn toggle_status(&mut self) {
        if let Some(i) = self.state.selected() {
            if self.mp3_files.len() -1 >= i {
                self.mp3_files[i].selected = !self.mp3_files[i].selected
            } else {
                self.state.select_first();
                self.toggle_status();
            }
        }
    }

    pub fn PromptSelection(&mut self) {
        self.state.select_first();
        self.toggle_status();
    }

    // Function to get MP3 files from a folder
    fn get_mp3_files_from_dir<P: AsRef<Path>>(folder_path: P) -> Vec<SoundItem> {

        let mut mp3_files = Vec::new();
        if let Ok(entries) = fs::read_dir(folder_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension == "mp3" {
                            if let Some(file_name) = path.file_name() {
                                mp3_files.push(SoundItem {name : file_name.to_string_lossy().into_owned(), selected : false, local_volume : 0.0});
                            }
                        }
                    }
                }
            }
        }
        mp3_files
    }

    pub fn update(&mut self) {
        self.mp3_files = SoundList::get_mp3_files_from_dir(self.current_dir.clone())
    }
}

impl StatefulWidget for SoundList {
    type State = ListState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        
        //println!("{:?}", state.selected());
        let mp3_list = List::new(self.get_list_items())
        .block(
            Block::bordered()
            .style(Style::default())
            .fg(match self.selected {
                true => {Color::Yellow},
                false => {Color::White},
            })
            .title(
                Line::from(match state.selected() {
                    Some(_) => {"MP3 Files - Selected"},
                    None => {"MP3 Files"}
                })
                .fg(match self.selected {
                    true => {Color::Yellow},
                    false => {Color::White},
                }))
                .border_style(Style::default()).fg(match self.selected {
                    true => {Color::Yellow},
                    false => {Color::White}
            })
            .title(
                Title::from(match state.selected() {
                    Some(_) => {"| <Enter> Play | <Space> Pause | <Backspace> Remove | <Shift> + ▲ ▼ Volume |"},
                    None => {""}
                })
                .alignment(Alignment::Center)
                .position(block::Position::Bottom)
            )
            .title(
                Title::from(match state.selected() {
                    Some(_) => {format!("{} Vol : {}",self.currently_playing.clone(), self.volume)}
                    None => {"-".to_string()}
                })
                .alignment(Alignment::Right)
                .position(block::Position::Top)
            )
            )
            .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(mp3_list, area, buf, &mut self.state.clone());
    }
}

