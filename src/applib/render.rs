use crate::interact_mod::component::SoundItem;
use crate::interact_mod::*;
use crate::interact_mod::component::DMXInput;
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
                            } else if input.input_mode {
                                "| Press <Enter> or <Esc> to validate |"
                            } else {
                                ""
                            }
                        } else if  sound_list.selected && !sound_list.sound_files.is_empty() {
                            "| Press <Shift> + ◄ ► to change Tab | Press <Enter> to enter the Sound List |"
                        } else if input.is_selected {
                            "| Press <Shift> + ◄ ► to change Tab | Press <Enter> to edit Path |"
                        } else {
                            ""
                        }
                    }
                    Content::Osc(_ipinput1, ) => {
                        if self.is_used() {
                            "| Press <Enter> to confirm |"
                        } else {
                            "| Press <Shift> + ◄ ► to change Tab |"
                        }
                    },
                    Content::Dmx(..) => {
                        if self.is_used() {"||"} else {"| Press <Shift> + ◄ ► to change Tab | Press ◄ ► to navigate between DMX Channel | "}
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
                        Content::Osc(ipinput1, ) => {
                            if self.is_used() {
                                ""
                            } else if ipinput1.focus   {
                                "| Press <Enter> to edit | Press <Shift> + ▲ ▼ to navigate |"
                            } else {"| Press <Shift> + ▲ ▼ to navigate |"}
                        },
                        Content::Dmx(..) => {
                            if self.is_used() {""} else {"| Enter <0-9> to set to DMX value | Press <Backspace> to reset DMX Value | <CTRL> + ▲ ▼ to modify DMX Value by 10 | ▲ ▼ to modify DMX Value by 1 |"}
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

                input.clone()
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
            Content::Osc(listening_ip_input, ) => {
                match [listening_ip_input.focus].iter().find(|b| **b) {
                    Some(_) => {},
                    None => {listening_ip_input.focus = true},
                }
                let vert = Layout::vertical([Constraint::Fill(1), Constraint::Fill(3)]);
                let [tab_content, _tab_footer] = vert.areas(tab_content);
                let ip_input_areas =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
                let [listening_input_area, _] = ip_input_areas.areas(tab_content);
                listening_ip_input.clone().render(
                    listening_input_area,
                    buf,
                    &mut listening_ip_input.input,
                );
                
            },
            Content::Dmx(dimmer_input,r_input,v_input,b_input,adr,ip, dmx_status) => {
                match [dimmer_input.is_focused,r_input.is_focused,v_input.is_focused,b_input.is_focused].iter().find(|b| **b) {
                    Some(_) => {
                    },
                    None => dimmer_input.is_focused = true,
                }
                let vert = Layout::vertical([Constraint::Percentage(10),Constraint::Percentage(90)]);
                let [top, bottom] = vert.areas(tab_content);
                let head_hor = Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)]);
                let [addr_area,ip_area,active_dmx_area] = head_hor.areas(top);
                Paragraph::new(format!("{}",adr)).block(Block::bordered().title("DMX Adress").title_bottom("| Press <Alt> + ⬆️⬇️ to change DMX adress |").title_alignment(Alignment::Center)).centered().render(addr_area,buf );
                Paragraph::new(ip.clone()).block(Block::bordered().title("Interface Serial").title_alignment(Alignment::Center).title_position(block::Position::Top).title_bottom("| Hold <CTRL> to write in Serial field | Press <CTRL> + <Backspace> to delete char | Press <SHIFT> + <Backspace> to reset |")).centered().render(ip_area,buf );
                Paragraph::new(dmx_status.clone()).block(Block::bordered().title_top("DMX Status").title_alignment(Alignment::Center)).centered().wrap(Wrap {trim : true}).render(active_dmx_area,buf );
                
                let hor = Layout::horizontal(Constraint::from_percentages([25,25,25,25]));
                let [left,lmid,rmid,right] = hor.areas(bottom);
                dimmer_input.clone().render(left,buf,&mut dimmer_input.value);
                r_input.clone().render(lmid,buf ,&mut r_input.value );
                v_input.clone().render(rmid,buf ,&mut v_input.value );
                b_input.clone().render(right,buf ,&mut b_input.value );
                
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
                .title(match self.input_mode {
                    false => Line::from(state.to_string()).centered().fg(Color::White),
                    true => Line::from(format!("{} - Edit", state))
                        .centered()
                        .fg(Color::Yellow),
                })
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
                        self.title.to_string()
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
                        Some(_) => {format!("{}{}",if self.currently_playing.is_empty() {""} else {"Playing : "},self.currently_playing.clone())}
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

impl StatefulWidget for DMXInput {
    type State = u8;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let frame = Block::bordered()
            .title(self.title.clone())
            .style(if self.is_focused {
                Style::new().fg(Color::Yellow)
            } else {
                    Style::new().fg(Color::White)
                }).title_alignment(Alignment::Center)
            .title_position(block::Position::Top);
        
        let bar = Bar::default()
            .value(*state as u64);
        BarChart::default()
            .direction(Direction::Vertical)
            .max(255)
            .data(BarGroup::default().bars(&[bar]))
            .style(Style::new()
                .fg(if self.is_focused {Color::Yellow} else {Color::White}))
            .bar_width(frame.inner(area).width)
            .render(frame.inner(area),buf );
        frame.render(area,buf );
    }
}
