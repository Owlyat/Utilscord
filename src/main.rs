use std::time::Duration;
use applib::tab_mod::Content;
use applib::AppTab;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;
mod applib;

struct App {
    should_quit: bool,
    quit_key_code: Vec<KeyCode>,
    move_key_code: Vec<KeyCode>,
    interact_key_code: KeyCode,
    tabs: applib::AppTab,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            quit_key_code: vec![KeyCode::Char('q'), KeyCode::Esc],
            move_key_code: vec![KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down],
            interact_key_code: KeyCode::Enter,
            tabs: applib::AppTab::default(),
        }
    }
}

impl App {
    pub fn run(&mut self) {
        let mut terminal = ratatui::init();

        while !self.should_quit {

            terminal.draw(|frame : &mut Frame| {
                frame.render_stateful_widget(
                    self.tabs.tab[self.tabs.selected_tab].clone(),
                    frame.area(),
                    &mut self.tabs.tab[self.tabs.selected_tab].tab_name
                );

            }).unwrap();

            self.handle_events();

        }

        ratatui::restore();

    }

    fn handle_events(&mut self) {

        let timeout = Duration::from_millis(50);

        if !event::poll(timeout).unwrap() {
            return
        }

        if let Event::Key(key) = event::read().unwrap(){

            if key.kind == KeyEventKind::Press {
                // QUIT EVENT
                for keycode in self.quit_key_code.clone() {
                    if (key.code, key.modifiers) == (keycode, KeyModifiers::CONTROL) {
                        self.should_quit = true
                    }
                }
                if (key.modifiers, key.code) == (KeyModifiers::CONTROL, KeyCode::Char('c')) {
                    self.should_quit = true
                }
                // MOVE EVENT
                for move_key_code in self.move_key_code.clone() {
                    if key.code == move_key_code && key.modifiers == KeyModifiers::SHIFT {

                        if let crate::applib::tab_mod::Content::MainMenu(soundlist,input) = &self.tabs.tab[self.tabs.selected_tab].content {
                            if !input.input_mode && soundlist.currently_playing.is_empty() {
                                match key.code {
                                    KeyCode::Up => {self.tabs.content_previous();},
                                    KeyCode::Down => {self.tabs.content_next();},
                                        KeyCode::Left => self.tabs.previous(),
                                        KeyCode::Right => self.tabs.next(),
                                        _ => panic!("you should not be here!"),
                                }
                            }
                        } else if let crate::applib::tab_mod::Content::None = &self.tabs.tab[self.tabs.selected_tab].content {
                            match key.code {
                                KeyCode::Up => {self.tabs.content_previous();},
                                KeyCode::Down => {self.tabs.content_next();},
                                    KeyCode::Left => self.tabs.previous(),
                                    KeyCode::Right => self.tabs.next(),
                                    _ => panic!("you should not be here!"),
                            }
                        }
                    }
                }


                    self.tabs.interact(key.code, key.modifiers);
            }
            

        }
    }
    
}


fn main() {
    let _ = App::default().run();
}
