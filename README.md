# Project Epod: Digital iPod 5th Generation (Video) Replica

A lightweight, ultra-low-resource local media player written in **Rust** replicating the physical aesthetic, interface, navigation mechanics, and feature set of the classic **Apple iPod 5th Generation (iPod Video)**.

---

## What's New & Highlights

### 1. "Album Cover" Display Screen Theme with Real-Time Blur & Brightness Controls
* In **Settings $\to$ Color Theme $\to$ Album Cover**:
  * Transforms the entire virtual LCD screen background into the **active song's album artwork** (or generative gradient for retro demo tracks).
  * **Configurable Blur Level** (`0 px` to `30 px`): Turn the Click Wheel to adjust the background blur from pin-sharp to soft frosted glass.
  * **Configurable Brightness Level** (`10%` to `100%`): Turn the Click Wheel to dial in the perfect brightness for optimal text contrast and aesthetics.
  * Automatic contrast-adaptive frosted glass UI styling across all menus, split preview panes, lists, and Now Playing views.

### 2. HD Album Cover Art on Discord Rich Presence
* When clicking on your Discord profile card, Discord now displays the **high-resolution song album cover art** directly inside the Rich Presence card.
* Asynchronously queries and matches official high-res album artwork for local music tracks (plus retro cover art for procedural tracks) and caches them for instant playback response.

### 2. Discord Display Modes
* In **Settings $\to$ Discord**:
  * **`App Only`** : *"Project Epod"* / *"Listening to music"*
  * **`Song Title`** : Displays the song title with active live progress bar
  * **`Artist`** : Displays the artist name
  * **`Song - Artist`** : Displays *"Song Title - Artist"* with animated countdown progress bar
* Seamlessly connects to your custom Discord Application ID (`1540631275717656648`) showing **`Listening to Epod`** in Discord server lists.

### 3. Mini On-Screen Back Button (`◀`)
* Interactive **`◀`** back button badge directly on the **bottom-left corner of the virtual LCD screen**.
* Click it directly with your mouse to step back to the previous screen without having to move your cursor down to the physical Click Wheel's MENU button.

### 4. Screen & Wheel Scoped Mouse Scrolling
* You can scroll with your mouse wheel **both when hovering over the capacitive Click Wheel AND when hovering anywhere over the virtual LCD screen**.
* Scrolling is cleanly constrained to only fire when the cursor is positioned over the Epod widget.

### 5. 10-Band Graphic Equalizer with Frequencies & Sub-Labels
* **10 ISO Octave Frequency Bands**:
  * **32 Hz** : `Sub` (Sub Bass)
  * **64 Hz** : `Low` (Low Bass)
  * **125 Hz** : `Bass` (Bass)
  * **250 Hz** : `MBass` (Mid Bass)
  * **500 Hz** : `LMid` (Low Mid)
  * **1 kHz** : `Mid` (Midrange)
  * **2 kHz** : `HMid` (High Mid)
  * **4 kHz** : `Pres` (Presence)
  * **8 kHz** : `Treb` (Treble)
  * **16 kHz** : `Air` (Brilliance/Air)
* Color-coded acoustic range sub-labels (Bass = Red, Midrange = Green, Treble = Blue).
* -12 dB to +12 dB real-time multi-band DSP audio adjustment. Rotate the Click Wheel to adjust gain; press **Select** (center button) to advance to the next band.

### 6. Instant Settings & Sources Auto-Persistence
* All configurations are **automatically saved to disk immediately upon any change (`epod_settings.json` and `epod_sources.json`)**:
  * Discord RPC enabled status, display mode & custom client ID
  * Chassis Theme (White / Piano Black / U2 Edition / Album Cover)
  * Album Cover theme blur & brightness settings
  * 10-Band Equalizer settings & Active Preset
  * Volume level
  * Music source directories
  * Brightness & Backlight timer
  * Clicker sound setting
  * Shuffle / Repeat / Sound Check modes
  * Main Menu customizer visibility toggles

### 7. Desktop Widget & Dynamic Scaling
* **Instant Startup (< 20ms)**: Fast background async scanner indexes folders without blocking the UI.
* **Single-Instance Protection**: Named Windows mutex prevents accidental multiple duplicate instances.
* **Proportional 4:3 LCD Screen Scaling**: Screen, artwork, text, and click wheel scale dynamically when resized.
* **Edge Drag Resizing**: Drag any window border or corner to resize freely.
* **Interactive 3.5mm Aux Port**: Clicking the headphone jack acts as a hardware shortcut straight to **Settings**.
* **Real Album Artwork & A-Z Sorted Songs**: Automatic extraction of embedded ID3 artwork and local `cover.jpg` / `folder.jpg` images with smooth texture rendering.

---

## Location & Binary
- Project root: `C:\Users\jaepe\OneDrive\Documents\PROJ\Epod`
- Release executable: `C:\Users\jaepe\OneDrive\Documents\PROJ\Epod\target\release\epod.exe`
