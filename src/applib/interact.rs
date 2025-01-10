#[path = "components.rs"]
pub mod component;
use component::DMXInput;
use component::IPInput;
use component::MusicState;
use component::{Content, Input, SoundList, Tab};
use core::panic;
use lofty::file::AudioFile;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use rosc::OscBundle;
use rosc::OscMessage;
use rosc::OscPacket;
use std::env;
use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
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
                soundlist.currently_playing = "".to_owned();
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
                soundlist.currently_playing = "".to_owned();
                let (mts, wtr) = mpsc::channel();
                let (wts, mtr) = mpsc::channel();
                let volume_changer_channel = mts.clone();
                let volume_changer_channel_2 = mts.clone();
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
                let fade_in_duration = Duration::from_secs(fadein as u64);
                let fade_out_duration = Duration::from_secs(fadeout as u64);
                let song_duration = lofty::read_from_path(&std::path::Path::new(
                    format!(
                        "{}/{}",
                        soundlist.current_dir,
                        soundlist.sound_files[index] //A CHANGER
                            .name
                    )
                    .as_mut_str(),
                ))
                .unwrap()
                .properties()
                .duration();
                if fadein != 0.0 {
                    let end_volume = soundlist.volume;
                    let start_volume = soundlist.volume - soundlist.volume;

                    // Spawn a new thread for the fading process
                    thread::spawn(move || {
                        let now = Instant::now();

                        loop {
                            let volume = interpolate_value(
                                now.elapsed(),
                                fade_in_duration,
                                start_volume,
                                end_volume,
                            );

                            if volume_changer_channel
                                .send(component::MusicState::VolumeChanged(volume))
                                .is_err()
                            {
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
                                    end_volume,
                                );

                                if volume_changer_channel_2
                                    .send(component::MusicState::VolumeChanged(volume))
                                    .is_err()
                                {
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

                soundlist.play(wtr, wts, index); // A CHANGER
            }
        }
    }

    pub fn interact(&mut self, key: KeyCode, keymod: KeyModifiers) {
        match &mut self.tabs[self.selected_tab].content {
            Content::MainMenu(soundlist, inputfield) => {
                // Selected inputfield
                if inputfield.is_selected {
                    input_field_logic(inputfield, key, keymod, soundlist);
                }
                // Selected Soundlist no fade edit
                else if soundlist.selected && !soundlist.editingfades {
                    if let Some(i) = soundlist.state.selected() {
                        match key {
                            KeyCode::Up => {
                                if keymod == KeyModifiers::SHIFT {
                                    // Modify local volume
                                    match soundlist.modify_local_volume(
                                        i,
                                        soundlist.get_local_volume_of_selected_item() + 0.01,
                                    ) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            panic!("{}", e)
                                        }
                                    }
                                    // Check if the playing music is the file selected on the list
                                    if soundlist.currently_playing == soundlist.sound_files[i].name
                                    {
                                        if let Some(sender) = &mut self.sender {
                                            // Send local volume
                                            match sender.send(
                                                component::MusicState::LocalVolumeChanged(
                                                    soundlist.get_local_volume_of_selected_item(),
                                                ),
                                            ) {
                                                _ => {}
                                            }
                                        }
                                    }
                                } else {
                                    soundlist.previous_song();
                                }
                            }
                            KeyCode::Down => {
                                if keymod == KeyModifiers::SHIFT {
                                    // Modify local volume
                                    match soundlist.modify_local_volume(
                                        i,
                                        (soundlist.get_local_volume_of_selected_item() - 0.01)
                                            .clamp(-2.0, 2.0),
                                    ) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            panic!("{}", e)
                                        }
                                    }
                                    // Check if the playing music is the file selected on the list
                                    if soundlist.currently_playing == soundlist.sound_files[i].name
                                    {
                                        if let Some(sender) = &mut self.sender {
                                            // Send local volume
                                            match sender.send(
                                                component::MusicState::LocalVolumeChanged(
                                                    soundlist.get_local_volume_of_selected_item(),
                                                ),
                                            ) {
                                                _ => {}
                                            }
                                        }
                                    }
                                } else {
                                    soundlist.next_song();
                                }
                            }

                            KeyCode::Enter => {
                                if let Some(sender) = &mut self.sender {
                                    match sender.send(component::MusicState::Remove) {
                                        _ => {}
                                    };
                                }
                                soundlist.currently_playing = "".to_owned();
                                let (mts, wtr) = mpsc::channel();
                                let (wts, mtr) = mpsc::channel();
                                let volume_changer_channel = mts.clone();
                                let volume_changer_channel_2 = mts.clone();
                                self.sender = Some(mts);
                                self.receiver = Some(mtr);
                                let fadein = soundlist.sound_files[i].fade_tab_content[0]
                                    .input
                                    .as_mut_str();
                                let fadein = fadein.trim().parse::<f32>().unwrap_or(0.0);
                                let fadeout = soundlist.sound_files[i].fade_tab_content[1]
                                    .input
                                    .as_mut_str();
                                let fadeout = fadeout.trim().parse::<f32>().unwrap_or(0.0);
                                let fade_in_duration = Duration::from_secs(fadein as u64);
                                let fade_out_duration = Duration::from_secs(fadeout as u64);
                                let song_duration = lofty::read_from_path(&std::path::Path::new(
                                    format!(
                                        "{}/{}",
                                        soundlist.current_dir, soundlist.sound_files[i].name
                                    )
                                    .as_mut_str(),
                                ))
                                .unwrap()
                                .properties()
                                .duration();
                                if fadein != 0.0 {
                                    let end_volume = soundlist.volume;
                                    let start_volume = soundlist.volume - soundlist.volume;

                                    // Spawn a new thread for the fading process
                                    thread::spawn(move || {
                                        let now = Instant::now();

                                        loop {
                                            let volume = interpolate_value(
                                                now.elapsed(),
                                                fade_in_duration,
                                                start_volume,
                                                end_volume,
                                            );

                                            if volume_changer_channel
                                                .send(component::MusicState::VolumeChanged(volume))
                                                .is_err()
                                            {
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
                                        let fade_out_start_point =
                                            song_duration - fade_out_duration;
                                        let now = Instant::now();
                                        loop {
                                            if now.elapsed() >= fade_out_start_point {
                                                let volume = interpolate_value(
                                                    song_duration.abs_diff(now.elapsed()),
                                                    fade_out_duration,
                                                    start_volume,
                                                    end_volume,
                                                );

                                                if volume_changer_channel_2
                                                    .send(component::MusicState::VolumeChanged(
                                                        volume,
                                                    ))
                                                    .is_err()
                                                {
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

                                soundlist.play(wtr, wts, i);
                            }

                            KeyCode::Esc => {
                                if soundlist.editingfades {
                                    soundlist.editingfades = false;
                                }
                                soundlist.unselect();
                            }

                            KeyCode::Backspace | KeyCode::Delete => {
                                soundlist.currently_playing = "".to_owned();
                                if let Some(x) = &mut self.sender {
                                    match x.send(component::MusicState::Remove) {
                                        _ => {}
                                    };
                                }
                            }

                            KeyCode::Char(' ') => {
                                if let Some(sender) = &mut self.sender {
                                    match sender.send(component::MusicState::PlayResume) {
                                        _ => {}
                                    };
                                }
                            }

                            KeyCode::Char('+') => {
                                soundlist.volume += 0.01;
                                soundlist.volume = soundlist.volume.clamp(0.0, 2.0);
                                if let Some(sender) = &mut self.sender {
                                    match sender.send(component::MusicState::VolumeChanged(
                                        soundlist.volume,
                                    )) {
                                        _ => {}
                                    };
                                }
                            }

                            KeyCode::Char('-') => {
                                soundlist.volume -= 0.01;
                                soundlist.volume = soundlist.volume.clamp(0.0, 2.0);
                                if let Some(sender) = &mut self.sender {
                                    match sender.send(component::MusicState::VolumeChanged(
                                        soundlist.volume,
                                    )) {
                                        _ => {}
                                    };
                                }
                            }

                            KeyCode::Char('f') => {
                                soundlist.toggle_fade_edition();
                            }

                            KeyCode::Char(c) => {
                                if matches!(
                                    c,
                                    '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'
                                ) {
                                    let index = c.to_string().parse::<usize>().unwrap();
                                    soundlist.select_song(index);
                                    if keymod == KeyModifiers::CONTROL {
                                        if let Some(sender) = &mut self.sender {
                                            match sender.send(component::MusicState::Remove) {
                                                _ => {}
                                            };
                                        }
                                        soundlist.currently_playing = "".to_owned();
                                        let (mts, wtr) = mpsc::channel();
                                        let (wts, mtr) = mpsc::channel();
                                        let volume_changer_channel = mts.clone();
                                        let volume_changer_channel_2 = mts.clone();
                                        self.sender = Some(mts);
                                        self.receiver = Some(mtr);
                                        let fadein = soundlist.sound_files[i].fade_tab_content[0]
                                            .input
                                            .as_mut_str();
                                        let fadein = fadein.trim().parse::<f32>().unwrap_or(0.0);
                                        let fadeout = soundlist.sound_files[i].fade_tab_content[1]
                                            .input
                                            .as_mut_str();
                                        let fadeout = fadeout.trim().parse::<f32>().unwrap_or(0.0);
                                        let fade_in_duration = Duration::from_secs(fadein as u64);
                                        let fade_out_duration = Duration::from_secs(fadeout as u64);
                                        let song_duration =
                                            lofty::read_from_path(&std::path::Path::new(
                                                format!(
                                                    "{}/{}",
                                                    soundlist.current_dir,
                                                    soundlist.sound_files[i].name
                                                )
                                                .as_mut_str(),
                                            ))
                                            .unwrap()
                                            .properties()
                                            .duration();
                                        if fadein != 0.0 {
                                            let end_volume = soundlist.volume;
                                            let start_volume = soundlist.volume - soundlist.volume;

                                            // Spawn a new thread for the fading process
                                            thread::spawn(move || {
                                                let now = Instant::now();

                                                loop {
                                                    let volume = interpolate_value(
                                                        now.elapsed(),
                                                        fade_in_duration,
                                                        start_volume,
                                                        end_volume,
                                                    );

                                                    if volume_changer_channel
                                                        .send(component::MusicState::VolumeChanged(
                                                            volume,
                                                        ))
                                                        .is_err()
                                                    {
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
                                                let fade_out_start_point =
                                                    song_duration - fade_out_duration;
                                                let now = Instant::now();
                                                loop {
                                                    if now.elapsed() >= fade_out_start_point {
                                                        let volume = interpolate_value(
                                                            song_duration.abs_diff(now.elapsed()),
                                                            fade_out_duration,
                                                            start_volume,
                                                            end_volume,
                                                        );

                                                        if volume_changer_channel_2
                                                            .send(
                                                                component::MusicState::VolumeChanged(
                                                                    volume,
                                                                ),
                                                            )
                                                            .is_err()
                                                        {
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

                                        soundlist.play(wtr, wts, i);
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else if soundlist.state.selected() == None
                        && fs::read_dir(soundlist.current_dir.clone()).is_ok()
                    {
                        match key {
                            KeyCode::Enter => soundlist.prompt_selection(),
                            _ => {}
                        }
                    }
                //EDITING FADES
                } else if soundlist.editingfades {
                    fade_tab(soundlist, key, keymod);
                }
            }

            Content::OSC(listening_ip_input, remote_ip_input) => {
                let inputs = vec![listening_ip_input, remote_ip_input];
                match key {
                    //OSC
                    //IPINPUT INPUT LOGIC
                    KeyCode::Enter => {
                        for input in inputs {
                            if input.focus {
                                match input.toggle_edit_mode() {
                                    Ok(rcv) => {
                                        self.osc_receiver = Some(rcv);
                                    }
                                    Err(()) => {
                                        self.osc_receiver = None;
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char(char) => {
                        if !matches!(
                            char,
                            '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' | ':' | '.'
                        ) {
                            return;
                        }
                        for input in inputs {
                            if input.edit_mode {
                                input.enter_char(char)
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        for input in inputs {
                            if input.edit_mode {
                                if keymod == KeyModifiers::CONTROL {
                                    for _ in input.clone().input.chars() {
                                        input.move_cursor_right();
                                        input.delete_char();
                                    }
                                } else {
                                    input.delete_char()
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        for input in inputs {
                            if input.edit_mode {
                                input.move_cursor_left()
                            }
                        }
                    }
                    KeyCode::Right => {
                        for input in inputs {
                            if input.edit_mode {
                                input.move_cursor_right()
                            }
                        }
                    }
                    _ => {}
                }
            }
            Content::DMX(dimmer, r, v, b, adr) => match key {
                KeyCode::Up => {
                    if keymod == KeyModifiers::ALT {
                        **adr = adr.saturating_add(1);
                        return;
                    }
                    let mut vec = vec![dimmer, r, v, b];
                    vec.iter_mut()
                        .map(|e| {
                            if e.is_focused && keymod != KeyModifiers::SHIFT {
                                if keymod == KeyModifiers::CONTROL {
                                    e.increment(10);
                                } else {
                                    e.increment(1);
                                }
                            }
                        })
                        .count();
                }
                KeyCode::Down => {
                    if keymod == KeyModifiers::ALT {
                        **adr = adr.saturating_sub(1);
                        return;
                    }
                    let mut vec = vec![dimmer, r, v, b];
                    vec.iter_mut()
                        .map(|e| {
                            if e.is_focused && keymod != KeyModifiers::SHIFT {
                                if keymod == KeyModifiers::CONTROL {
                                    e.decrement(10);
                                } else {
                                    e.decrement(1);
                                }
                            }
                        })
                        .count();
                }
                KeyCode::Char(x) => {
                    if matches!(x, '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9') {
                        let mut vec = vec![dimmer, r, v, b];
                        let focused = vec.iter_mut().find(|e| e.is_focused).unwrap();
                        match format!("{}{}", focused.value, x).parse::<u8>() {
                            Ok(v) => focused.value = v,
                            Err(_e) => focused.value = 255,
                        }
                    }
                }
                KeyCode::Backspace => {
                    let mut vec = vec![dimmer, r, v, b];
                    let focused = vec.iter_mut().find(|e| e.is_focused).unwrap();
                    focused.value = 0;
                }
                _ => {}
            },
        }
    }
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
                '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' | '.' => {
                    si.fade_tab_content[0].enter_char(char_to_insert);
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
                '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' | '.' => {
                    si.fade_tab_content[1].enter_char(char_to_insert);
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
                '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' => {
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

fn interpolate_value(elapsed: Duration, fade: Duration, start: f32, end: f32) -> f32 {
    let t = elapsed.as_secs_f32();

    if t <= fade.as_secs_f32() {
        // Interpolate from start to end during fade
        start + (end - start) * (t / fade.as_secs_f32()).clamp(0.0, 1.0)
    } else {
        end
    }
}
