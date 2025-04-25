# Utilscord
 
## A TUI based program for live event / Theatre in pure rust

### Features

Tabs|Shortcuts
--- | ---
Change tab | Shift + Arrow left/Right
Navigate | Shift + Arrow Up/Down OR J/K
Quit | Ctrl + C / Ctrl + Q


| Sound List | Shortcuts |
|:-----------:|:---------:|
|play| Enter |
|pause| Space |
|stop| Backspace |
|Local volume| Shift + Up/Down Arrow |
|Edit Song Properties| F |
|Go to Song| 0 1 2 3 4 5 6 7 8 9 (based on number) |
|Play song| CTRL + 1 2 3 4 5 6 7 8 9 (based on number) |
|General volume| +/- |
|Exit| Esc |


| OSC | Mapping |
--- | ---
/Utilscord/Play/0 | Play the song specified number (here 0)
/Utilscord/Play/Next | Play the next song, work even if the SoundPlayer is not selected
/Utilscord/Play/Previous | Play the previous song, work even if the SoundPlayer is not selected
/Utilscord/Stop | Stop the current playing song
/Utilscord/LocalVolume/Selected (Float) | Will set the local volume of the selected song
/Utilscord/LocalVolume/0 (Float) | Will set the local volume of the song specified number (here 0)
/Utilscord/Volume (float) | Will set the general volume of the app
/DMXChan/{number between 1 to 512}/{Number between 0 to 255} | Will set a DMX Channel to the desired value ex: /DMXChan/55/255

### To do (in priority from top to bottom) :
- [x] A General volume + A dedicated assignable volume for each sound
- [x] Adjustable Fade In and Fade Out for each sound
- [x] OSC Controls
- [x] Adjustable song trim for each sound
- [x] Implement [dmx](https://docs.rs/dmx/latest/dmx/) to do basic lighting
- [ ] Implement [dante-control-rs](https://docs.rs/dante-control-rs/0.8.2/dante_control_rs/) to manage dante devices

### How to use

1. Git clone https://github.com/Owlyat/Utilscord
2. cd Utilscord
3. Cargo run
4. (Optional) if you want a binary just use 'Cargo Build --release'
