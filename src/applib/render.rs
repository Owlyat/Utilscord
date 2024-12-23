use crate::applib::tab_mod::SoundItem;
use crate::applib::*;
use ratatui::prelude::*;
use ratatui::widgets::block::Title;
use ratatui::widgets::*;

impl StatefulWidget for Tab {
    type State = Content;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let main_tab_border = Block::bordered()
            .title(
                Title::from(state.to_string())
                    .position(block::Position::Top)
                    .alignment(Alignment::Center),
            )
            .fg(Color::White)
            .title(
                Title::from(match &self.content {
                    Content::MainMenu(sound_list, input) => {
                        if self.is_used() {
                            if sound_list.state.selected().is_some() {
                                "| Press <Esc> to quit Sound List |"
                            } else {
                                if input.input_mode {
                                    "| Press <Enter> or <Esc> to validate |"
                                } else {
                                    ""
                                }
                            }
                        } else {
                            if sound_list.selected && sound_list.sound_files.len() != 0 {
                                "| Press <Shift> + ◄ ► to change Tab | Press <Enter> to enter the Sound List |"
                            } else {
                                if input.is_selected {
                                    "| Press <Shift> + ◄ ► to change Tab | Press <Enter> to edit Path |"
                                } else {
                                    ""
                                }
                            }
                        }
                    }
                    Content::OSC(_ipinput1, _ipinput2) => {
                        if self.is_used() {
                            "| Press <Enter> to confirm |"
                        } else {
                            "| Press <Shift> + ◄ ► to change Tab |"
                        }
                    }
                })
                .position(block::Position::Bottom)
                .alignment(Alignment::Left),
            )
            .fg(Color::White)
            .title(
                Title::from(
                    match &self.content {
                        Content::MainMenu(sound_list,_input ) =>  {
                            if self.is_used() {
                                if sound_list.state.selected().is_some() {
                                    "| Press ▲ ▼ to navigate |"
                                } else {
                                    ""
                                }
                            } else {"| Press <Shift> + ▲ ▼ to navigate |"}
                        },
                        Content::OSC(ipinput1,ipinput2 ) => {
                            if self.is_used() {
                                ""
                            } else {
                                if ipinput1.focus {
                                    "| Press <Enter> to edit | Press <Shift> + ▲ ▼ to navigate |"
                                } else {
                                    if ipinput2.focus {
                                        "| Press <Enter> to edit | Press <Shift> + ▲ ▼ to navigate |"
                                    } else {"| Press <Shift> + ▲ ▼ to navigate |"}
                                }
                            }
                        }
                    }
                )
                .position(block::Position::Bottom)
                .alignment(Alignment::Right),
            )
            .style(Style::default())
            .white()
            .bg(Color::Black);
        let tab_content = main_tab_border.inner(area);
        main_tab_border.render(area, buf);

        match state {
            Content::MainMenu(sound_list, input) => {
                let vert = Layout::vertical([Constraint::Length(3), Constraint::Fill(3)]);

                let [tab_content, tab_footer] = vert.areas(tab_content);

                input
                    .clone()
                    .render(tab_content, buf, &mut input.input_field_title);

                if sound_list.editingfades {
                    sound_list.clone().sound_files[sound_list.state.selected().unwrap()]
                        .clone()
                        .render(
                            tab_footer,
                            buf,
                            &mut sound_list.sound_files[sound_list.state.selected().unwrap()]
                                .edit_tab_selected,
                        );
                } else {
                    sound_list
                        .clone()
                        .render(tab_footer, buf, &mut sound_list.state);
                }
            }
            Content::OSC(listening_ip_input, remote_ip_input) => {
                let vert = Layout::vertical([Constraint::Fill(1), Constraint::Fill(3)]);
                let [tab_content, _tab_footer] = vert.areas(tab_content);
                let ip_input_areas =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
                let [listening_input_area, remote_input_area] = ip_input_areas.areas(tab_content);
                listening_ip_input.clone().render(
                    listening_input_area,
                    buf,
                    &mut listening_ip_input.input,
                );
                remote_ip_input
                    .clone()
                    .render(remote_input_area, buf, &mut remote_ip_input.input)
            }
        }
    }
}

impl StatefulWidget for Input {
    type State = String;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Paragraph::new(match self.input_mode {
            false => Line::from(self.input).fg(Color::White),
            true => Line::from(self.input).fg(Color::Yellow),
        })
        .style(match self.input_mode {
            false => Style::default(),
            true => Style::default().yellow(),
        })
        .block(
            Block::bordered()
                .title(Line::from(match self.input_mode {
                    false => Line::from(state.to_string()).centered().fg(Color::White),
                    true => Line::from(format!("{} - Edit", state.to_string()))
                        .centered()
                        .fg(Color::Yellow),
                }))
                .fg(match self.is_selected {
                    true => Color::Yellow,
                    false => Color::White,
                }),
        )
        .render(area, buf);
    }
}

impl StatefulWidget for IPInput {
    type State = String;
    //IPINPUT RENDER
    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        Paragraph::new(self.input)
            .style(if self.edit_mode || self.focus {
                Style::default().yellow()
            } else {
                Style::default().white()
            })
            .block(
                Block::bordered()
                    .title(if self.edit_mode {
                        format!("{} - Edit", self.title)
                    } else {
                        format!("{}", self.title)
                    })
                    .title_alignment(Alignment::Center)
                    .title_position(block::Position::Top),
            )
            .render(area, buf);
    }
}

impl StatefulWidget for SoundItem {
    type State = usize;
    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut usize) {
        let popup = Block::bordered()
            .title(
                Title::from("Fades".white())
                    .alignment(Alignment::Center)
                    .position(block::Position::Top),
            )
            .fg(Color::White)
            .title(
                Title::from("| Press <F> or <ESC> To Go Back |")
                    .alignment(Alignment::Center)
                    .position(block::Position::Bottom),
            );
        let content = popup.inner(area);
        popup.render(area, buf);
        let layout = Layout::vertical([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
        ]);
        let [fade_in_area, fade_out_area, trim_in_area] = layout.areas(content);

        let mut copy = self.fade_tab_content.clone();

        copy[0]
            .clone()
            .render(fade_in_area, buf, &mut copy[0].input_field_title);
        copy[1]
            .clone()
            .render(fade_out_area, buf, &mut copy[1].input_field_title);
        copy[2]
            .clone()
            .render(trim_in_area, buf, &mut copy[2].input_field_title);
    }
}

impl StatefulWidget for SoundList {
    type State = ListState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        //println!("{:?}", state.selected());
        let sound_list = List::new(self.get_list_items())
        .block(
            Block::bordered()
            .style(Style::default())
            .fg(match self.selected {
                true => {Color::Yellow},
                false => {Color::White},
            })
            .title(
                Line::from(match state.selected() {
                    Some(_) => {"Sound List - Selected"},
                    None => {"Sound List"}
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
                    Some(_) => {"| <Enter> Play | <Space> Pause | <Backspace> Remove | <Shift> + ▲ ▼ Local Volume | +/- General Volume |"},
                    None => {""}
                })
                .alignment(Alignment::Center)
                .position(block::Position::Bottom)
            )
            .title(
                Title::from(
                    match state.selected() {
                        Some(_) => {format!("Playing : {}",self.currently_playing.clone())}
                        None => {"-".to_string()}
                    })
                .alignment(Alignment::Right)
                .position(block::Position::Top)
            )
            .title(
                Title::from(match state.selected() {
                    Some(_) => {format!("|General Volume : {:.2}|", self.volume)}
                    None => {"".to_string()}
                })
                .alignment(Alignment::Right)
                .position(block::Position::Bottom)
            )
            )
            .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(sound_list, area, buf, &mut self.state.clone());
    }
}
