#[path = "components.rs"]
pub mod component;
use component::DMXInput;
use component::IPInput;
use component::MusicState;
use component::{Content, Input, SoundList, Tab};
use core::panic;
use open_dmx::DMX_CHANNELS;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::MouseEvent;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use rosc::OscBundle;
use rosc::OscMessage;
use rosc::OscPacket;
use rosc::OscType;
use std::env;
use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::vec;

use crate::dmx::DMXHandler;

#[derive(Debug)]
pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub selected_tab: usize,
    pub sender: Option<Sender<MusicState>>,
    pub receiver: Option<Receiver<f32>>,
    pub osc_receiver: Option<Receiver<OscPacket>>,
    pub dmx_handler: DMXHandler,
}

impl TabManager {
    pub fn get_selected_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.selected_tab]
    }
    #[allow(dead_code)]
    pub fn get_selected_tab(&self) -> &Tab {
        &self.tabs[self.selected_tab]
    }
    pub fn next(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.tabs.len()
    }

    pub fn previous(&mut self) {
        self.selected_tab = (self.selected_tab + self.tabs.len() - 1) % self.tabs.len()
    }

    pub fn osc_bundle_interaction(&mut self, _osc_bundle: OscBundle) {}

    pub fn osc_message_interaction(&mut self, osc_message: OscMessage) -> Result<(), String> {
        let osc_path: Vec<&str> = osc_message.addr.split("/").collect();
        match osc_path[2] {
            "LocalVolume" | "Volume" | "Stop" | "Play" => {
                match self.osc_message_soundlist(&osc_message, &osc_path) {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        if let Content::Osc(ipinput) = &mut self.tabs[1].content {
                            ipinput.update_info(format!("Error : {e}",));
                        }
                        return Err(e);
                    }
                }
            }

            "DMXChan" => match self.osc_message_dmx(&osc_message, &osc_path) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if let Content::Osc(ipinput) = &mut self.tabs[1].content {
                        ipinput.update_info(format!("Error : {e}",));
                    }
                    return Err(e);
                }
            },
            _ => {}
        }
        Err(format!("Invalid OSC path : {}", osc_path[2]))
    }

    fn osc_message_dmx(
        &mut self,
        osc_message: &OscMessage,
        osc_path: &[&str],
    ) -> Result<(), String> {
        if osc_path[2] == "DMXChan" {
            match (osc_path[3], &osc_message.args[0]) {
                (chan_str, osc_type)
                    if chan_str
                        .parse::<usize>()
                        .is_ok_and(|channel| open_dmx::check_valid_channel(channel).is_ok()) =>
                {
                    if let OscType::Int(dmx_value_i32) = osc_type {
                        if (0..=255).contains(dmx_value_i32) {
                            if let Ok(dmx_channel) = chan_str.parse::<usize>() {
                                if (1..DMX_CHANNELS).contains(&dmx_channel) {
                                    if let Some(dmx_connection) =
                                        &mut self.dmx_handler.dmx_connection_option
                                    {
                                        if dmx_connection.check_agent().is_ok()
                                            && dmx_connection
                                                .set_channel(dmx_channel, *dmx_value_i32 as u8)
                                                .is_ok()
                                        {
                                            if let Content::Osc(ipinput) = &mut self.tabs[1].content
                                            {
                                                ipinput.update_info(format!(
                                                    "Channel {} set to {}",
                                                    dmx_channel, dmx_value_i32
                                                ));
                                            }
                                            return Ok(());
                                        }
                                    } else {
                                        return Err(String::from("No DMX connection found !"));
                                    }
                                } else {
                                    return Err(format!("{} is not in range 1..=512", dmx_channel));
                                }
                            } else {
                                return Err(format!("Cannot Convert {} to usize", chan_str));
                            }
                        } else {
                            return Err(format!("{} is not in range 0..=255", dmx_value_i32));
                        }
                    } else if let OscType::Float(dmx_value_f32) = osc_type {
                        if (0..=255).contains(&(dmx_value_f32.round() as i32)) {
                            if let Ok(dmx_channel) = chan_str.parse::<usize>() {
                                if (1..DMX_CHANNELS).contains(&dmx_channel) {
                                    if let Some(dmx_connection) =
                                        &mut self.dmx_handler.dmx_connection_option
                                    {
                                        if dmx_connection.check_agent().is_ok()
                                            && dmx_connection
                                                .set_channel(
                                                    dmx_channel,
                                                    dmx_value_f32.round() as u8,
                                                )
                                                .is_ok()
                                        {
                                            if let Content::Osc(ipinput) = &mut self.tabs[1].content
                                            {
                                                ipinput.update_info(format!(
                                                    "Channel {} set to {}",
                                                    dmx_channel, dmx_value_f32
                                                ));
                                            }
                                            return Ok(());
                                        }
                                    } else {
                                        return Err(String::from("No DMX connection found !"));
                                    }
                                } else {
                                    return Err(format!("{} is not in range 1..=512", dmx_channel));
                                }
                            } else {
                                return Err(format!("Cannot Convert {} to usize", chan_str));
                            }
                        } else {
                            return Err(format!("{} is not in range 0..=255", dmx_value_f32));
                        }
                    } else {
                        return Err(format!("{:?} is not an Int !", osc_type));
                    }
                }
                _ => {
                    return Err(format!(
                        "Invalid channel or value : {} <= This must be between 1 and 512 {:?}",
                        osc_path[3], osc_message.args[0]
                    ))
                }
            }
        }
        Err(format!("Invalid DMX Channel : {}", osc_path[2]))
    }

    fn osc_message_soundlist(
        &mut self,
        osc_message: &OscMessage,
        osc_path: &[&str],
    ) -> Result<(), String> {
        if osc_path[2] == "LocalVolume" {
            if let Content::MainMenu(soundlist, input) = &mut self.tabs[0].content {
                if soundlist.sound_files.is_empty() {
                    return Err("No Sound Files in the Sound List".to_owned());
                }
                let value = osc_message.args.first();
                if let Some(value) = value {
                    if let Some(new_volume) = value.clone().float() {
                        if osc_path.get(3).is_some() {
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

                                match soundlist.state.selected() {
                                    Some(v) => v,
                                    None => {
                                        soundlist.prompt_selection();
                                        soundlist.state.selected().unwrap()
                                    }
                                }
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
                            let res1 = soundlist.modify_local_volume(index, new_volume);
                            let mut res2 = Ok(());
                            if soundlist.currently_playing == soundlist.sound_files[index].name {
                                if let Some(sender) = &mut self.sender {
                                    // Send local volume
                                    res2 = sender.send(component::MusicState::LocalVolumeChanged(
                                        soundlist.get_local_volume_of_item_index(index),
                                    ));
                                }
                            }
                            match (res1, res2) {
                            (Ok(_), Ok(_)) => {
                                            if let Content::Osc(ipinput) = &mut self.tabs[1].content
                                            {
                                                ipinput.update_info(format!("Local Volume of item {} Changed to {}",index,new_volume));
                                            }

                                return Ok(())
                            },
                            (Err(e), Ok(())) => {
                                return Err(format!("Error while modifying local volume : {e}"))
                            }
                            (Ok(_), Err(e)) => return Err(format!(
                                "Error while modifying local volume of a playing sound file : {e}"
                            )),
                            (Err(e1), Err(e2)) => {
                                return Err(format!("Error while modifying local volume of a playing sound file with two errors : \n{}\n{}",e1,e2))
                            }
                        }
                        } else {
                            return Err("Missing OSC path index of item, ex : Selected | 1 | 125 <= Sound Index".to_owned());
                        }
                    } else {
                        return Err(format!("Argument Value {:?} is not a Float", value));
                    }
                } else {
                    return Err("Argument Value not provided".to_owned());
                }
            }
        }
        if osc_path[2] == "Volume" {
            if let Content::MainMenu(soundlist, _input) = &mut self.tabs[0].content {
                if soundlist.sound_files.is_empty() {
                    return Err("No Sound Files in the Sound List".to_owned());
                }

                if osc_message.args.is_empty() {
                    return Err("No Volume Value provided".to_owned());
                }
                for arg in osc_message.args.clone() {
                    if let Some(v) = arg.clone().float() {
                        soundlist.volume = v;
                        if let Some(sender) = &mut self.sender {
                            let _ =
                                sender.send(component::MusicState::VolumeChanged(soundlist.volume));
                            {
                                if let Content::Osc(ipinput) = &mut self.tabs[1].content {
                                    ipinput.update_info(format!("General Volume set to {v}"));
                                }
                                return Ok(());
                            };
                        }
                    } else {
                        return Err(format!("{:?}, is not a float", arg));
                    }
                }
            } else {
                return Err("Cannot modify Volume if there is no Main Menu".to_owned());
            }
        }
        if osc_path[2] == "Stop" {
            if let Content::MainMenu(soundlist, _input) = &mut self.tabs[0].content {
                soundlist.currently_playing.clear();
                if let Some(x) = &mut self.sender {
                    let _ = x.send(component::MusicState::Remove);
                    {
                        if let Content::Osc(ipinput) = &mut self.tabs[1].content {
                            ipinput.update_info("Sound Stoped".to_string());
                        }
                        return Ok(());
                    };
                }
            } else {
                return Err("Can't Stop a Sound if there is no Main Menu".to_owned());
            }
        }
        if osc_path[2] == "Play" {
            if let Content::MainMenu(soundlist, input) = &mut self.tabs[0].content {
                if soundlist.sound_files.is_empty() {
                    return Err("No Sound Files in the Sound List".to_owned());
                }

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

                    match soundlist.state.selected() {
                        Some(_) => {
                            soundlist.next_song();
                            soundlist.state.selected().unwrap()
                        }
                        None => {
                            soundlist.prompt_selection();
                            soundlist.state.selected().unwrap()
                        }
                    }
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

                    match soundlist.state.selected() {
                        Some(_) => {
                            soundlist.previous_song();
                            soundlist.state.selected().unwrap()
                        }
                        None => {
                            soundlist.prompt_selection();
                            soundlist.state.selected().unwrap()
                        }
                    }
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
                if soundlist.sound_files.is_empty() {
                    return Err(format!(
                        "There are no Sound Files found in {}",
                        soundlist.current_dir
                    ));
                }
                if let Some(sender) = &mut self.sender {
                    let _ = sender.send(component::MusicState::Remove);
                    {};
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
            return Ok(());
        }
        Err(format!("Invalid OSC path : {}", osc_path[2]))
    }

    pub fn handle_event(
        &mut self,
        event: ratatui::crossterm::event::Event,
        file_manager: &mut ratatui_explorer::FileExplorer,
    ) {
        match event {
            ratatui::crossterm::event::Event::FocusGained => self.handle_event_focus_gained(),
            ratatui::crossterm::event::Event::FocusLost => self.handle_event_focus_lost(),
            ratatui::crossterm::event::Event::Key(key_event) => {
                self.handle_keys_event(key_event, file_manager)
            }
            ratatui::crossterm::event::Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
            }
            ratatui::crossterm::event::Event::Paste(content) => self.handle_event_paste(content),
            ratatui::crossterm::event::Event::Resize(x, y) => self.handle_event_resize(x, y),
        }
    }
    fn handle_keys_event(
        &mut self,
        key: KeyEvent,
        file_manager: &mut ratatui_explorer::FileExplorer,
    ) {
        match &mut self.tabs[self.selected_tab].content {
            Content::MainMenu(sound_list, input) if key.kind == KeyEventKind::Press => {
                if input.is_selected {
                    input_field_logic(input, key.code, key.modifiers, sound_list, file_manager);
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
                                            if sender
                                                .send(component::MusicState::LocalVolumeChanged(
                                                    sound_list.get_local_volume_of_selected_item(),
                                                ))
                                                .is_ok()
                                            {
                                                return;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k' | 'K') => {
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
                                            if sender
                                                .send(component::MusicState::LocalVolumeChanged(
                                                    sound_list.get_local_volume_of_selected_item(),
                                                ))
                                                .is_ok()
                                            {
                                                return;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j' | 'J') => {
                                    sound_list.next_song();
                                    return;
                                }
                                KeyCode::Enter if key.kind == KeyEventKind::Press => {
                                    if let Some(sender) = &mut self.sender {
                                        let _ = sender.send(component::MusicState::Remove);
                                        {};
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
                                        if x.send(component::MusicState::Remove).is_ok() {
                                            return;
                                        }
                                    }
                                }

                                KeyCode::Char(' ') => {
                                    if let Some(sender) = &mut self.sender {
                                        let _ =
                                            sender.send(component::MusicState::PlayResume).is_ok();
                                        return;
                                    }
                                }

                                KeyCode::Char('+') => {
                                    sound_list.volume += 0.01;
                                    sound_list.volume = sound_list.volume.clamp(0.0, 2.0);
                                    if let Some(sender) = &mut self.sender {
                                        let _ = sender.send(component::MusicState::VolumeChanged(
                                            sound_list.volume,
                                        ));
                                        return;
                                    }
                                }

                                KeyCode::Char('-') => {
                                    sound_list.volume -= 0.01;
                                    sound_list.volume = sound_list.volume.clamp(0.0, 2.0);
                                    if let Some(sender) = &mut self.sender {
                                        if sender
                                            .send(component::MusicState::VolumeChanged(
                                                sound_list.volume,
                                            ))
                                            .is_ok()
                                        {
                                            return;
                                        };
                                    }
                                }

                                KeyCode::Char('f') => {
                                    sound_list.toggle_fade_edition();
                                    return;
                                }

                                KeyCode::Char(c) => {
                                    if c.is_ascii_digit() {
                                        let index = c.to_string().parse::<usize>().unwrap();
                                        sound_list.select_song(index);
                                        if key.modifiers == KeyModifiers::CONTROL {
                                            if let Some(sender) = &mut self.sender {
                                                let _ = sender.send(component::MusicState::Remove);
                                                {};
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
                    }
                }
            }
            Content::Osc(listening_ip_input) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Enter => {
                    if listening_ip_input.focus {
                        if let Ok(rcv) = listening_ip_input.toggle_edit_mode() {
                            self.osc_receiver = Some(rcv);
                        } else {
                            self.osc_receiver = None;
                        }
                    }
                }
                KeyCode::Char(char) => {
                    if !matches!(char, '0'..='9' | ':' | '.') {
                        return;
                    }
                    if listening_ip_input.edit_mode {
                        listening_ip_input.enter_char(char)
                    }
                }
                KeyCode::Backspace => {
                    if listening_ip_input.edit_mode {
                        if key.modifiers == KeyModifiers::CONTROL {
                            for _ in listening_ip_input.clone().input.chars() {
                                listening_ip_input.move_cursor_right();
                                listening_ip_input.delete_char();
                            }
                        } else {
                            listening_ip_input.delete_char();
                        }
                    }
                }
                KeyCode::Left => {
                    if listening_ip_input.edit_mode {
                        listening_ip_input.move_cursor_left();
                    }
                }
                KeyCode::Right => {
                    if listening_ip_input.edit_mode {
                        listening_ip_input.move_cursor_right();
                    }
                }
                _ => (),
            },
            Content::Dmx(f1, f2, f3, f4, adr, serial, _dmx_status)
                if key.kind == KeyEventKind::Press =>
            {
                match key.code {
                    KeyCode::Up => {
                        if self.dmx_handler.dmx_connection_option.is_some() {
                            if key.modifiers == KeyModifiers::ALT {
                                if **adr != open_dmx::DMX_CHANNELS - 3 {
                                    **adr = adr.saturating_add(1);
                                    self.dmx_handler
                                        .update_dmx(&mut self.tabs[self.selected_tab].content);
                                    return;
                                }
                                return;
                            }
                            let mut dmx_faders = [f1, f2, f3, f4];
                            dmx_faders.iter_mut().for_each(|dmx_fader| {
                                if dmx_fader.is_focused {
                                    if key.modifiers == KeyModifiers::CONTROL {
                                        dmx_fader.increment(10);
                                    } else {
                                        dmx_fader.increment(1);
                                    }
                                }
                            });
                            self.dmx_handler
                                .update_dmx(&mut self.tabs[self.selected_tab].content);
                        }
                    }
                    KeyCode::Down => {
                        if self.dmx_handler.dmx_connection_option.is_some() {
                            if key.modifiers == KeyModifiers::ALT {
                                if **adr != 1 {
                                    **adr = adr.saturating_sub(1);
                                    self.dmx_handler
                                        .update_dmx(&mut self.tabs[self.selected_tab].content);
                                    return;
                                }
                                return;
                            }
                            let mut dmx_faders = [f1, f2, f3, f4];
                            dmx_faders.iter_mut().for_each(|dmx_fader| {
                                if dmx_fader.is_focused {
                                    if key.modifiers == KeyModifiers::CONTROL {
                                        dmx_fader.decrement(10);
                                    } else {
                                        dmx_fader.decrement(1);
                                    }
                                }
                            });
                            self.dmx_handler
                                .update_dmx(&mut self.tabs[self.selected_tab].content);
                        }
                    }
                    KeyCode::Char(char) => {
                        if self.dmx_handler.dmx_connection_option.is_some()
                            && key.modifiers == KeyModifiers::NONE
                        {
                            let mut dmx_faders = [f1, f2, f3, f4];

                            if char.is_ascii_digit() {
                                dmx_faders
                                    .iter_mut()
                                    .find(|d| d.is_focused)
                                    .iter_mut()
                                    .for_each(|d| {
                                        d.value = format!("{}{}", d.value, char)
                                            .parse::<u8>()
                                            .unwrap_or(255);
                                    });
                                self.dmx_handler
                                    .update_dmx(&mut self.tabs[self.selected_tab].content);
                                return;
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
                            self.dmx_handler
                                .update_dmx(&mut self.tabs[self.selected_tab].content);
                            return;
                        }
                        if self.dmx_handler.dmx_connection_option.is_none() {
                            serial.push(char);
                        }
                    }
                    KeyCode::Backspace => {
                        if self.dmx_handler.dmx_connection_option.is_none() {
                            match key.modifiers {
                                KeyModifiers::CONTROL => {
                                    serial.clear();
                                }
                                _ => {
                                    serial.pop();
                                }
                            }
                            return;
                        }
                        // If dmx connection :
                        if self.dmx_handler.dmx_connection_option.is_some() {
                            if key.modifiers == KeyModifiers::ALT {
                                **adr = 1;
                                return;
                            }
                            let mut dmx_faders = [f1, f2, f3, f4];
                            let focused = dmx_faders
                                .iter_mut()
                                .find(|dmx_fader| dmx_fader.is_focused)
                                .unwrap();
                            focused.value = 0;
                            self.dmx_handler
                                .update_dmx(&mut self.tabs[self.selected_tab].content);
                        }
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
    fn handle_event_paste(&mut self, content: String) {
        match &mut self.tabs[self.selected_tab].content {
            Content::MainMenu(_sound_list, input) => {
                if input.is_selected && input.input_mode {
                    input.input.push_str(&content);
                }
            }
            Content::Osc(ipinput) => {
                if ipinput.focus && ipinput.edit_mode {
                    ipinput.input.push_str(&content);
                }
            }
            Content::Dmx(_dmxinput, _dmxinput1, _dmxinput22, _dmxinput33, _, _, _) => (),
        }
    }
    fn handle_event_resize(&mut self, _x: u16, _y: u16) {}
}

impl Default for TabManager {
    fn default() -> Self {
        let args: Vec<_> = env::args().collect();
        let mut dmx_content = Content::Dmx(
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
        );
        let dmx_handler = match DMXHandler::open_dmx(&mut dmx_content) {
            Ok(dmx_handler) => dmx_handler,
            Err(empty_dmx_handler) => empty_dmx_handler,
        };
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
                            input_field_title: "Path to Sound Files".to_owned(),
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
                    content: Content::Osc(IPInput::new("Host IP:PORT".to_owned())),
                },
                Tab {
                    content: dmx_content,
                },
            ],
            selected_tab: 0,
            sender: None,
            receiver: None,
            osc_receiver: None,
            dmx_handler,
        };
        //CLI
        if args.len() > 1 {
            app.tabs[0].next_content_element();
        }
        app
    }
}

fn input_field_logic(
    inputfield: &mut Input,
    key: KeyCode,
    keymod: KeyModifiers,
    soundlist: &mut SoundList,
    file_manager: &mut ratatui_explorer::FileExplorer,
) {
    //Normal Mode
    if !inputfield.input_mode && key == KeyCode::Enter {
        inputfield.toggle();
    }
    //Edit Mode
    else if inputfield.input_mode {
        match key {
            KeyCode::Enter => {
                inputfield.toggle();
                soundlist.current_dir =
                    file_manager.current().path().to_string_lossy().into_owned();
                soundlist.update();
                inputfield.input = soundlist.current_dir.clone();
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
            KeyCode::Left => file_manager.handle(ratatui_explorer::Input::Left).unwrap(),
            KeyCode::Right => file_manager.handle(ratatui_explorer::Input::Right).unwrap(),
            KeyCode::Esc => inputfield.toggle(),
            KeyCode::Up => file_manager.handle(ratatui_explorer::Input::Up).unwrap(),
            KeyCode::Down => file_manager.handle(ratatui_explorer::Input::Down).unwrap(),
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
                '0'..='9' | '.' => {
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
                '0'..='9' | '.' => {
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
            KeyCode::Char(char_to_insert) => {
                if let '0'..='9' = char_to_insert {
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
            }
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

#[cfg(test)]
mod test {
    use rosc::OscType;

    use super::*;

    fn test_osc(adr: &str, arg: Option<OscType>, expected: &str) {
        let mut t = TabManager::default();
        let res = t.osc_message_interaction(OscMessage {
            addr: adr.to_owned(),
            args: if arg.is_some() {
                vec![arg.unwrap()]
            } else {
                vec![]
            },
        });
        assert_eq!(res, Err(expected.to_string()))
    }

    #[test]
    fn no_dmx_connection() {
        test_osc(
            "/OscControl/DMXChan/8",
            Some(OscType::Int(255)),
            "No DMX connection found !",
        );
    }
    #[test]
    fn dmx_addr_overflow() {
        test_osc(
            "/OscControl/DMXChan/513",
            Some(OscType::Int(200)),
            "Invalid channel or value : 513 <= This must be between 1 and 512 Int(200)",
        );
    }
    #[test]
    fn dmx_addr_0() {
        test_osc(
            "/OscControl/DMXChan/0",
            Some(OscType::Int(200)),
            "Invalid channel or value : 0 <= This must be between 1 and 512 Int(200)",
        );
    }
    #[test]
    fn dmx_value_overflow() {
        test_osc(
            "/OscControl/DMXChan/5",
            Some(OscType::Int(256)),
            "256 is not in range 0..=255",
        );
    }
    #[test]
    fn dmx_value_wrong_type() {
        test_osc(
            "/OscControl/DMXChan/5",
            Some(OscType::Float(10.5)),
            "Float(10.5) is not an Int !",
        );
    }
    #[test]
    fn dmx_osc_path_verification() {
        test_osc(
            "/OscControl/DMX/200",
            Some(OscType::Int(10)),
            "Invalid OSC path : DMX",
        );
    }
    #[test]
    fn no_volume_value() {
        test_osc("/OscControl/Volume", None, "No Volume Value provided");
    }
    #[test]
    fn volume_wrong_type() {
        test_osc(
            "/OscControl/Volume",
            Some(OscType::Int(1)),
            "Int(1), is not a float",
        );
    }
    #[test]
    fn localvolume_no_value() {
        test_osc(
            "/OscControl/LocalVolume/Selected",
            None,
            "Argument Value not provided",
        );
    }
    #[test]
    fn localvolume_missing_index() {
        test_osc(
            "/OscControl/LocalVolume",
            Some(OscType::Float(10.0)),
            "Missing OSC path index of item, ex : Selected | 1 | 125 <= Sound Index",
        );
    }

    #[test]
    fn localvolume_wrong_type() {
        test_osc(
            "/OscControl/LocalVolume/Selected",
            Some(OscType::String("Test".to_owned())),
            "Argument Value String(\"Test\") is not a Float",
        );
    }
}
