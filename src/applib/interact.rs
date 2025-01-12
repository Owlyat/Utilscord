#[path = "components.rs"]
pub mod component;
use component::DMXInput;
use component::IPInput;
use component::MusicState;
use component::{Content, Input, SoundList, Tab};
use core::panic;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::MouseEvent;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use rosc::OscBundle;
use rosc::OscMessage;
use rosc::OscPacket;
use std::env;
use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::vec;

pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub selected_tab: usize,
    pub sender: Option<Sender<MusicState>>,
    pub receiver: Option<Receiver<f32>>,
    pub osc_receiver: Option<Receiver<OscPacket>>,
}

impl TabManager {
    fn new() -> Self {
        let args: Vec<_> = env::args().collect();
        let mut app = TabManager {
            tabs: vec![
                Tab {
                    content: Content::MainMenu(
                        SoundList::from_dir(if args.len() > 1 {
                            args[1].clone()
                        } else {
                            String::new()
                        }),
                        Input {
                            input_field_title: "Path".to_owned(),
                            is_selected: true,
                            input: if args.len() > 1 {
                                args[1].clone()
                            } else {
                                String::new()
                            },
                            ..Default::default()
                        },
                    ),
                },
                Tab {
                    content: Content::OSC(
                        IPInput::new("Host IP:PORT".to_owned()),
                        IPInput::new("Host IP:PORT".to_owned()),
                    ),
                },
                Tab {
                    content: Content::DMX(
                        DMXInput {
                            title: "Dimmer".to_owned(),
                            ..Default::default()
                        },
                        DMXInput {
                            title: "R".to_owned(),
                            ..Default::default()
                        },
                        DMXInput {
                            title: "V".to_owned(),
                            ..Default::default()
                        },
                        DMXInput {
                            title: "B".to_owned(),
                            ..Default::default()
                        },
                        Box::new(1),
                        String::new(),
                        String::new(),
                    ),
                },
            ],
            selected_tab: 0,
            sender: None,
            receiver: None,
            osc_receiver: None,
        };
        //CLI
        if args.len() > 1 {
            app.tabs[0].next_content_element();
        }
        app
    }

    pub fn get_selected_tab(&mut self) -> &mut Tab {
        &mut self.tabs[self.selected_tab]
    }
    pub fn next(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.tabs.len()
    }

    pub fn previous(&mut self) {
        self.selected_tab = (self.selected_tab + self.tabs.len() - 1) % self.tabs.len()
    }

    pub fn osc_bundle_interaction(&mut self, _osc_bundle: OscBundle) {}

    pub fn osc_message_interaction(&mut self, osc_message: OscMessage) {
        let osc_path: Vec<&str> = osc_message.addr.split("/").collect();
        if osc_path[2] == "LocalVolume" {
            if let Content::MainMenu(soundlist, input) = &mut self.tabs[0].content {
                for arg in osc_message.args.clone() {
                    match arg.float() {
                        Some(new_volume) => {
                            let index = if osc_path[3] == "Selected" {
                                if self.selected_tab != 0 {
                                    self.selected_tab = 0;
                                }
                                if input.is_selected {
                                    input.is_selected = false;
                                }
                                if !soundlist.selected {
                                    soundlist.selected = true;
                                }
                                let value = match soundlist.state.selected() {
                                    Some(v) => v,
                                    None => {
                                        soundlist.prompt_selection();
                                        soundlist.state.selected().unwrap()
                                    }
                                };
                                value
                            } else {
                                match osc_path[3].parse::<usize>() {
                                    Ok(number) => {
                                        if soundlist.sound_files.len() <= number {
                                            0
                                        } else {
                                            number
                                        }
                                    }
                                    Err(_e) => 0,
                                }
                            };

                            match soundlist.modify_local_volume(index, new_volume) {
                                Ok(_) => {}
                                Err(_) => {}
                            };
                            if soundlist.currently_playing == soundlist.sound_files[index].name {
                                if let Some(sender) = &mut self.sender {
                                    // Send local volume
                                    match sender.send(component::MusicState::LocalVolumeChanged(
                                        soundlist.get_local_volume_of_item_index(index),
                                    )) {
                                        _ => {}
                                    }
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
        }
        if osc_path[2] == "Volume" {
            if let Content::MainMenu(soundlist, _input) = &mut self.tabs[0].content {
                for arg in osc_message.args.clone() {
                    match arg.float() {
                        Some(v) => {
                            soundlist.volume = v;
                            if let Some(sender) = &mut self.sender {
                                match sender
                                    .send(component::MusicState::VolumeChanged(soundlist.volume))
                                {
                                    _ => {}
                                };
                            }
                        }
                        None => {}
                    }
                }
            }
        }
        if osc_path[2] == "Stop" {
            if let Content::MainMenu(soundlist, _input) = &mut self.tabs[0].content {
                soundlist.currently_playing.clear();
                if let Some(x) = &mut self.sender {
                    match x.send(component::MusicState::Remove) {
                        _ => {}
                    };
                }
            }
        }
        if osc_path[2] == "Play" {
            if let Content::MainMenu(soundlist, input) = &mut self.tabs[0].content {
                let index = if osc_path[3] == "Next" {
                    if self.selected_tab != 0 {
                        self.selected_tab = 0;
                    }
                    if input.is_selected {
                        input.is_selected = false;
                    }
                    if !soundlist.selected {
                        soundlist.selected = true;
                    }
                    let value = match soundlist.state.selected() {
                        Some(_) => {
                            soundlist.next_song();
                            soundlist.state.selected().unwrap()
                        }
                        None => {
                            soundlist.prompt_selection();
                            soundlist.state.selected().unwrap()
                        }
                    };
                    value
                } else if osc_path[3] == "Previous" {
                    if self.selected_tab != 0 {
                        self.selected_tab = 0;
                    }
                    if input.is_selected {
                        input.is_selected = false;
                    }
                    if !soundlist.selected {
                        soundlist.selected = true;
                    }
                    let value = match soundlist.state.selected() {
                        Some(_) => {
                            soundlist.previous_song();
                            soundlist.state.selected().unwrap()
                        }
                        None => {
                            soundlist.prompt_selection();
                            soundlist.state.selected().unwrap()
                        }
                    };
                    value
                } else {
                    match osc_path[3].parse::<usize>() {
                        Ok(number) => {
                            if soundlist.sound_files.len() <= number {
                                0
                            } else {
                                number
                            }
                        }
                        Err(_e) => 0,
                    }
                };
                if soundlist.sound_files.len() == 0 {
                    return;
                }
                if let Some(sender) = &mut self.sender {
                    match sender.send(component::MusicState::Remove) {
                        _ => {}
                    };
                }
                soundlist.currently_playing.clear();
                let (mts, wtr) = mpsc::channel();
                let (wts, mtr) = mpsc::channel();
                self.sender = Some(mts);
                self.receiver = Some(mtr);
                let fadein = soundlist.sound_files[index] //A CHANGER
                    .fade_tab_content[0]
                    .input
                    .as_mut_str();
                let fadein = fadein.trim().parse::<f32>().unwrap_or(0.0);
                let fadeout = soundlist.sound_files[index] //A CHANGER
                    .fade_tab_content[1]
                    .input
                    .as_mut_str();
                let fadeout = fadeout.trim().parse::<f32>().unwrap_or(0.0);
                let fade_in_duration = match fadein {
                    x if x > 0.0 => Some(Duration::from_secs(fadein as u64)),
                    _ => None,
                };
                let fade_out_duration = match fadeout {
                    x if x > 0.0 => Some(Duration::from_secs(fadeout as u64)),
                    _ => None,
                };

                soundlist.play(wtr, wts, index, fade_in_duration, fade_out_duration);
            }
        }
    }

    pub fn handle_event(&mut self, event: ratatui::crossterm::event::Event) {
        match event {
            ratatui::crossterm::event::Event::FocusGained => self.handle_event_focus_gained(),
            ratatui::crossterm::event::Event::FocusLost => self.handle_event_focus_lost(),
            ratatui::crossterm::event::Event::Key(key_event) => self.handle_keys_event(key_event),
            ratatui::crossterm::event::Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
            }
            ratatui::crossterm::event::Event::Paste(content) => self.handle_event_paste(content),
            ratatui::crossterm::event::Event::Resize(x, y) => self.handle_event_resize(x, y),
        }
    }
    fn handle_keys_event(&mut self, key: KeyEvent) {
        match &mut self.tabs[self.selected_tab].content {
            Content::MainMenu(sound_list, input) if key.kind == KeyEventKind::Press => {
                if input.is_selected {
                    input_field_logic(input, key.code, key.modifiers, sound_list);
                    return;
                }
                if sound_list.selected {
                    if !sound_list.editingfades {
                        if let Some(index) = sound_list.state.selected() {
                            match key.code {
                                KeyCode::Up if key.modifiers == KeyModifiers::SHIFT => {
                                    match sound_list.modify_local_volume(
                                        index,
                                        sound_list.get_local_volume_of_selected_item() + 0.01,
                                    ) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            panic!("{}", e)
                                        }
                                    }
                                    // Check if the playing song is the file selected on the list
                                    // so it sends the new volume to the currently playing song
                                    if sound_list.currently_playing
                                        == sound_list.sound_files[index].name
                                    {
                                        if let Some(sender) = &mut self.sender {
                                            // Send local volume
                                            if let Ok(_) = sender.send(
                                                component::MusicState::LocalVolumeChanged(
                                                    sound_list.get_local_volume_of_selected_item(),
                                                ),
                                            ) {
                                                return;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Up => {
                                    sound_list.previous_song();
                                    return;
                                }
                                KeyCode::Down if key.modifiers == KeyModifiers::SHIFT => {
                                    // Modify local volume
                                    match sound_list.modify_local_volume(
                                        index,
                                        (sound_list.get_local_volume_of_selected_item() - 0.01)
                                            .clamp(-2.0, 2.0),
                                    ) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            panic!("{}", e)
                                        }
                                    }
                                    if sound_list.currently_playing
                                        == sound_list.sound_files[index].name
                                    {
                                        if let Some(sender) = &mut self.sender {
                                            if let Ok(_) = sender.send(
                                                component::MusicState::LocalVolumeChanged(
                                                    sound_list.get_local_volume_of_selected_item(),
                                                ),
                                            ) {
                                                return;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Down => {
                                    sound_list.next_song();
                                    return;
                                }
                                KeyCode::Enter if key.kind == KeyEventKind::Press => {
                                    if let Some(sender) = &mut self.sender {
                                        match sender.send(component::MusicState::Remove) {
                                            _ => {}
                                        };
                                    }
                                    sound_list.currently_playing.clear();
                                    let (mts, wtr) = mpsc::channel();
                                    let (wts, mtr) = mpsc::channel();
                                    self.sender = Some(mts);
                                    self.receiver = Some(mtr);
                                    let fadein = sound_list.sound_files[index].fade_tab_content[0]
                                        .input
                                        .as_mut_str();
                                    let fadein = fadein.trim().parse::<f32>().unwrap_or(0.0);
                                    let fadeout = sound_list.sound_files[index].fade_tab_content[1]
                                        .input
                                        .as_mut_str();
                                    let fadeout = fadeout.trim().parse::<f32>().unwrap_or(0.0);
                                    let fade_in_duration = match fadein {
                                        x if x > 0.0 => Some(Duration::from_secs(fadein as u64)),
                                        _ => None,
                                    };
                                    let fade_out_duration = match fadeout {
                                        x if x > 0.0 => Some(Duration::from_secs(fadeout as u64)),
                                        _ => None,
                                    };
                                    sound_list.play(
                                        wtr,
                                        wts,
                                        index,
                                        fade_in_duration,
                                        fade_out_duration,
                                    );
                                    return;
                                }
                                KeyCode::Esc if key.kind == KeyEventKind::Press => {
                                    if sound_list.editingfades {
                                        sound_list.editingfades = false;
                                    }
                                    sound_list.unselect();
                                    return;
                                }

                                KeyCode::Backspace | KeyCode::Delete => {
                                    sound_list.currently_playing.clear();
                                    if let Some(x) = &mut self.sender {
                                        if let Ok(_) = x.send(component::MusicState::Remove) {
                                            return;
                                        }
                                    }
                                }

                                KeyCode::Char(' ') => {
                                    if let Some(sender) = &mut self.sender {
                                        if let Ok(_) =
                                            sender.send(component::MusicState::PlayResume)
                                        {
                                        }
                                        return;
                                    }
                                }

                                KeyCode::Char('+') => {
                                    sound_list.volume += 0.01;
                                    sound_list.volume = sound_list.volume.clamp(0.0, 2.0);
                                    if let Some(sender) = &mut self.sender {
                                        match sender.send(component::MusicState::VolumeChanged(
                                            sound_list.volume,
                                        )) {
                                            _ => return,
                                        };
                                    }
                                }

                                KeyCode::Char('-') => {
                                    sound_list.volume -= 0.01;
                                    sound_list.volume = sound_list.volume.clamp(0.0, 2.0);
                                    if let Some(sender) = &mut self.sender {
                                        if let Ok(_) = sender.send(
                                            component::MusicState::VolumeChanged(sound_list.volume),
                                        ) {
                                            return;
                                        };
                                    }
                                }

                                KeyCode::Char('f') => {
                                    sound_list.toggle_fade_edition();
                                    return;
                                }

                                KeyCode::Char(c) => {
                                    if matches!(c, '0'..='9') {
                                        let index = c.to_string().parse::<usize>().unwrap();
                                        sound_list.select_song(index);
                                        if key.modifiers == KeyModifiers::CONTROL {
                                            if let Some(sender) = &mut self.sender {
                                                match sender.send(component::MusicState::Remove) {
                                                    _ => {}
                                                };
                                            }
                                            sound_list.currently_playing.clear();
                                            let (mts, wtr) = mpsc::channel();
                                            let (wts, mtr) = mpsc::channel();
                                            self.sender = Some(mts);
                                            self.receiver = Some(mtr);
                                            let fadein = sound_list.sound_files[index]
                                                .fade_tab_content[0]
                                                .input
                                                .as_mut_str();
                                            let fadein =
                                                fadein.trim().parse::<f32>().unwrap_or(0.0);
                                            let fadeout = sound_list.sound_files[index]
                                                .fade_tab_content[1]
                                                .input
                                                .as_mut_str();
                                            let fadeout =
                                                fadeout.trim().parse::<f32>().unwrap_or(0.0);
                                            let fade_in_duration = match fadein {
                                                x if x > 0.0 => {
                                                    Some(Duration::from_secs(fadein as u64))
                                                }
                                                _ => None,
                                            };
                                            let fade_out_duration = match fadeout {
                                                x if x > 0.0 => {
                                                    Some(Duration::from_secs(fadeout as u64))
                                                }
                                                _ => None,
                                            };

                                            sound_list.play(
                                                wtr,
                                                wts,
                                                index,
                                                fade_in_duration,
                                                fade_out_duration,
                                            );
                                            return;
                                        }
                                    }
                                }

                                _ => (),
                            }
                        } else if fs::read_dir(sound_list.current_dir.clone()).is_ok() {
                            match key.code {
                                KeyCode::Enter if key.kind == KeyEventKind::Press => {
                                    sound_list.prompt_selection();
                                    return;
                                }
                                _ => (),
                            }
                        }
                    }
                    if sound_list.editingfades {
                        fade_tab(sound_list, key.code, key.modifiers);
                        return;
                    }
                }
            }
            Content::OSC(listening_ip_input, remote_ip_input)
                if key.kind == KeyEventKind::Press =>
            {
                let inputs = vec![listening_ip_input, remote_ip_input];
                match key.code {
                    KeyCode::Enter => {
                        for input in inputs {
                            if input.focus {
                                if let Ok(rcv) = input.toggle_edit_mode() {
                                    self.osc_receiver = Some(rcv);
                                } else {
                                    self.osc_receiver = None;
                                }
                                return;
                            }
                        }
                    }
                    KeyCode::Char(char) => {
                        if !matches!(char, '0'..='9' | ':' | '.') {
                            return;
                        }
                        for input in inputs {
                            if input.edit_mode {
                                input.enter_char(char)
                            }
                        }
                        return;
                    }
                    KeyCode::Backspace => {
                        for input in inputs {
                            if input.edit_mode {
                                if key.modifiers == KeyModifiers::CONTROL {
                                    for _ in input.clone().input.chars() {
                                        input.move_cursor_right();
                                        input.delete_char();
                                    }
                                    return;
                                } else {
                                    input.delete_char();
                                    return;
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        for input in inputs {
                            if input.edit_mode {
                                input.move_cursor_left();
                                return;
                            }
                        }
                    }
                    KeyCode::Right => {
                        for input in inputs {
                            if input.edit_mode {
                                input.move_cursor_right();
                                return;
                            }
                        }
                    }
                    _ => (),
                }
            }
            Content::DMX(dimmer, r, v, b, adr, serial, _dmx_status)
                if key.kind == KeyEventKind::Press =>
            {
                match key.code {
                    KeyCode::Up => {
                        if key.modifiers == KeyModifiers::ALT {
                            if **adr != open_dmx::DMX_CHANNELS {
                                **adr = adr.saturating_add(1);
                                return;
                            }
                            return;
                        }
                        let mut dmx_faders = vec![dimmer, r, v, b];
                        dmx_faders
                            .iter_mut()
                            .map(|dmx_fader| {
                                if dmx_fader.is_focused {
                                    if key.modifiers == KeyModifiers::CONTROL {
                                        dmx_fader.increment(10);
                                    } else {
                                        dmx_fader.increment(1);
                                    }
                                }
                            })
                            .count();
                    }
                    KeyCode::Down => {
                        if key.modifiers == KeyModifiers::ALT {
                            if **adr != 1 {
                                **adr = adr.saturating_sub(1);
                                return;
                            }
                            return;
                        }
                        let mut dmx_faders = vec![dimmer, r, v, b];
                        dmx_faders
                            .iter_mut()
                            .map(|dmx_fader| {
                                if dmx_fader.is_focused {
                                    if key.modifiers == KeyModifiers::CONTROL {
                                        dmx_fader.decrement(10);
                                    } else {
                                        dmx_fader.decrement(1);
                                    }
                                }
                            })
                            .count();
                    }
                    KeyCode::Char(char) if key.modifiers.is_empty() => {
                        let mut dmx_faders = vec![dimmer, r, v, b];

                        if matches!(char, '0'..='9') {
                            let focused = dmx_faders
                                .iter_mut()
                                .find(|dmx_fader| dmx_fader.is_focused)
                                .unwrap();
                            match format!("{}{}", focused.value, char).parse::<u8>() {
                                Ok(v) => {
                                    focused.value = v;
                                    return;
                                }
                                Err(_e) => {
                                    focused.value = 255;
                                    return;
                                }
                            }
                        } else if matches!(char, 'f' | 'F') {
                            dmx_faders.iter_mut().for_each(|dmx_fader| {
                                if dmx_fader.is_focused {
                                    dmx_fader.value = 255;
                                }
                            });
                            return;
                        } else if matches!(char, '.' | 'r' | 'R') {
                            dmx_faders.iter_mut().for_each(|dmx_fader| {
                                if dmx_fader.is_focused {
                                    dmx_fader.value = 0;
                                }
                            });
                            return;
                        }
                    }
                    KeyCode::Char(char) if key.modifiers == KeyModifiers::CONTROL => {
                        serial.push(char);
                        return;
                    }
                    KeyCode::Backspace => {
                        if key.modifiers == KeyModifiers::CONTROL {
                            serial.pop();
                            return;
                        }
                        if key.modifiers == KeyModifiers::SHIFT {
                            serial.clear();
                            return;
                        }
                        let mut dmx_faders = vec![dimmer, r, v, b];
                        let focused = dmx_faders
                            .iter_mut()
                            .find(|dmx_fader| dmx_fader.is_focused)
                            .unwrap();
                        focused.value = 0;
                        return;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn handle_mouse_event(&mut self, _mouse: MouseEvent) {}
    fn handle_event_focus_gained(&mut self) {}
    fn handle_event_focus_lost(&mut self) {}
    fn handle_event_paste(&mut self, _content: String) {}
    fn handle_event_resize(&mut self, _x: u16, _y: u16) {}
}

impl Default for TabManager {
    fn default() -> Self {
        TabManager::new()
    }
}

fn input_field_logic(
    inputfield: &mut Input,
    key: KeyCode,
    keymod: KeyModifiers,
    soundlist: &mut SoundList,
) {
    //Normal Mode
    if inputfield.input_mode == false && key == KeyCode::Enter {
        inputfield.toggle();
        return;
    }
    //Edit Mode
    else if inputfield.input_mode {
        match key {
            KeyCode::Char('V') => {
                if keymod == KeyModifiers::CONTROL {
                    inputfield.paste();
                } else {
                    inputfield.enter_char('v');
                }
            }
            KeyCode::Enter => {
                inputfield.toggle();
                soundlist.current_dir = inputfield.input.clone();
                soundlist.update();
            }
            KeyCode::Backspace => {
                if keymod == KeyModifiers::CONTROL {
                    inputfield.input.clear();
                    inputfield.reset_cursor();
                    return;
                } else {
                    inputfield.delete_char()
                }
            }
            KeyCode::Left => inputfield.move_cursor_left(),
            KeyCode::Right => inputfield.move_cursor_right(),
            KeyCode::Esc => inputfield.toggle(),
            KeyCode::Char(to_insert) => inputfield.enter_char(to_insert),
            _ => {
                return;
            }
        }
    }
}

fn fade_tab(soundlist: &mut SoundList, key: KeyCode, keymod: KeyModifiers) {
    let si = &mut soundlist.sound_files[soundlist.state.selected().unwrap()];

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
            KeyCode::Char(char_to_insert) => match char_to_insert {
                '0'..'9' | '.' => {
                    if Duration::from_secs(
                        format!("{}{}", si.fade_tab_content[0].input, char_to_insert)
                            .parse::<u64>()
                            .unwrap(),
                    ) > si.max_duration
                    {
                        si.fade_tab_content[0].input = si.max_duration.as_secs().to_string();
                    } else {
                        si.fade_tab_content[0].enter_char(char_to_insert);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    if si.fade_tab_content[1].input_mode {
        // Editing Fade Out Input
        match key {
            KeyCode::Backspace => {
                if keymod == KeyModifiers::CONTROL {
                    si.fade_tab_content[1].input.clear();
                    si.fade_tab_content[1].reset_cursor();
                } else {
                    si.fade_tab_content[1].delete_char();
                }
            }
            KeyCode::Char(char_to_insert) => match char_to_insert {
                '0'..'9' | '.' => {
                    if Duration::from_secs(
                        format!("{}{}", si.fade_tab_content[1].input, char_to_insert)
                            .parse::<u64>()
                            .unwrap(),
                    ) > si.max_duration
                    {
                        si.fade_tab_content[1].input = si.max_duration.as_secs().to_string();
                    } else {
                        si.fade_tab_content[1].enter_char(char_to_insert);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    if si.fade_tab_content[2].input_mode {
        // Editing Trim In
        match key {
            KeyCode::Backspace => {
                if keymod == KeyModifiers::CONTROL {
                    si.fade_tab_content[2].input.clear();
                    si.fade_tab_content[2].reset_cursor();
                } else {
                    si.fade_tab_content[2].delete_char();
                }
            }
            KeyCode::Char(char_to_insert) => match char_to_insert {
                '0'..'9' => {
                    si.fade_tab_content[2].enter_char(char_to_insert);

                    match si.fade_tab_content[2].input.parse::<u64>() {
                        Ok(duration) => {
                            if si.max_duration.as_secs() > duration {
                                si.trim_in = Duration::from_secs(duration);
                            } else {
                                si.fade_tab_content[2].input =
                                    si.max_duration.as_secs().to_string();
                                si.trim_in = Duration::from_secs(0);
                            }
                        }
                        Err(_) => {
                            si.fade_tab_content[2].enter_char(char_to_insert);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    // Navigating between Fade Inputs
    match key {
        KeyCode::Backspace => {
            if !si.fade_tab_content[2].input_mode {
                soundlist.toggle_fade_edition();
            }
        }
        KeyCode::Up => {
            for i in si.fade_tab_content.clone() {
                if i.input_mode {
                    return;
                }
            }
            si.previous_fade_tab();
        }
        KeyCode::Down => {
            for i in si.fade_tab_content.clone() {
                if i.input_mode {
                    return;
                }
            }
            si.next_fade_tab();
        }
        KeyCode::Left => {}
        KeyCode::Right => {}
        KeyCode::Enter => {
            si.edit();
        }
        KeyCode::Char('f') => {
            soundlist.toggle_fade_edition();
        }
        KeyCode::Esc => {
            soundlist.toggle_fade_edition();
        }
        _ => {}
    }
}
