# Changelog

All notable changes to Vogix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.12.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.11.0...vogix-v0.12.0) (2026-09-25)


### ⚠ BREAKING CHANGES

* **home-manager:** login shells no longer run `vogix theme refresh`. A text-only login restores nothing: the theme's files persist through current-theme, and the VT palette and the machine's LEDs are restored at boot by the NixOS module's machine owner units, which a home-manager-only install does not have.
* **machine:** vogix.hardware.themeApply is removed. Machine hardware is declared in vogix.hardware.devices and applied by the machine owner units from the owner's published palette; a consumer that copied themeApply into programs.vogix.themeApply must drop that line.
* **reload:** config.toml's [hardware."<name>"] tables are now [hooks."<name>"]; a hand-written [hardware] section is no longer read.
* **desktop:** desktop.json no longer carries the single-bar `bar` object (schema 1's shape, mirrored from `bars.top` for one release), and the shell no longer synthesizes a four-edge table from it. A schema-1 desktop.json renders no bars; rebuilding regenerates the file as schema 2.

### Features

* **desktop:** `vogix desktop bar geometry` lists where each widget sits ([c51ea8a](https://github.com/i-am-logger/vogix/commit/c51ea8a32ccd6bbcdb33f3f48578e22fc3d68636))
* **desktop:** `vogix desktop privacy` reports what the PRIVACY cell shows ([d2b1b03](https://github.com/i-am-logger/vogix/commit/d2b1b037ce5fad9a7c7f0299b4ca0dd11491957a))
* **desktop:** `vogix desktop stats` and `privacy` report what those cells read ([3eedd52](https://github.com/i-am-logger/vogix/commit/3eedd52568ce8054a5e16c362ef87256eb0825a2))
* **desktop:** `vogix desktop vu`, and the VU meters checked against a −6 dBFS tone ([eed5c4f](https://github.com/i-am-logger/vogix/commit/eed5c4feed2452b775c54b9cc987ea68155cfbf9))
* **desktop:** custom cells run only while their bar is on screen ([cddadb6](https://github.com/i-am-logger/vogix/commit/cddadb6897a7bd2267b1a437d7e74b14aa4b5875))
* **desktop:** custom cells show a configured command's output on any bar ([967b531](https://github.com/i-am-logger/vogix/commit/967b53152192d2384cca2eee89f158ed6f5bfa99))
* **desktop:** desktop.json drops the schema-1 `bar` mirror; the shell reads schema 2 only ([ab5928d](https://github.com/i-am-logger/vogix/commit/ab5928d2c46493802cbe56fb4f1716ebb37ce979))
* **desktop:** fan-speed cells, and configurable GPU thresholds ([4907b52](https://github.com/i-am-logger/vogix/commit/4907b520f5e0a4ff07eec7b117b9b62b2563f781))
* **desktop:** home-manager runs `vogix desktop check` on the desktop.json it builds ([88c1716](https://github.com/i-am-logger/vogix/commit/88c1716a4f128162e2c3a17aa9080ab5d40359fa))
* **desktop:** measure NVIDIA and Intel GPUs, not only amdgpu ([735eb0f](https://github.com/i-am-logger/vogix/commit/735eb0f9e968c3b72113a82105ea1a957d9cb4f5))
* **desktop:** one widget registry types the layouts, the check and the shell ([fad8a73](https://github.com/i-am-logger/vogix/commit/fad8a7372984e5937e633b227025dc7f040ab3a4))
* **desktop:** the media transport returns to the bottom bar, without the track title ([dd82b31](https://github.com/i-am-logger/vogix/commit/dd82b315f60859514fb9f80bf6913daeccb87566))
* **desktop:** the widget registry says which bar orientation a widget renders on ([e0b64e7](https://github.com/i-am-logger/vogix/commit/e0b64e75842cfd4c955386fedfee78596a18db32))
* **input:** publish the keyboard lock state; the LANG cell's CAPS follows it ([afe2a82](https://github.com/i-am-logger/vogix/commit/afe2a82e9a89ba72d1df6770dac48e43abe03b8d))
* **machine:** `vogix machine serve local`, the VT palette and command-device owner ([c3a588a](https://github.com/i-am-logger/vogix/commit/c3a588af5c17dbda811fe96058792709b143c6ed))
* **machine:** event reactor, single-flight command runner and hidraw uevents ([a3f1309](https://github.com/i-am-logger/vogix/commit/a3f130978f7438ae01b3af0bcf93dc580e9c884e))
* **machine:** OpenRGB SDK wire format, model and codec ([94f242f](https://github.com/i-am-logger/vogix/commit/94f242f28c8911627a97a638a84f06b17d103c9d))
* **machine:** OpenRGB session state machine, selection and write plans ([0ba5f32](https://github.com/i-am-logger/vogix/commit/0ba5f32ad02467158f0af60d4c570147f2f93d2b))
* **machine:** publish the owner's palette after the state commit, and `vogix machine status` ([024af05](https://github.com/i-am-logger/vogix/commit/024af05099af9e4ba31c272efcbacb6841190807))
* **machine:** systemd notifications and the owners' typed status file ([02c940c](https://github.com/i-am-logger/vogix/commit/02c940c423fe14af45552d0605096bb594075e26))
* **machine:** the NixOS module declares the machine owner, typed devices and the owner units ([b624661](https://github.com/i-am-logger/vogix/commit/b6246610fb4aa17245702932d91f8ef7c1e663c7))
* **machine:** typed machine config and palette, and `vogix machine validate` ([8bca90b](https://github.com/i-am-logger/vogix/commit/8bca90b6569ab9d3c3dbdcc88a4176bc108cf78a))
* **machine:** vogix machine serve openrgb and a read-only vogix machine inspect ([fd01920](https://github.com/i-am-logger/vogix/commit/fd0192000bb8d206dc0928d6a3c611fdf6cafffa))
* **openrgb:** run the SDK server as Type=notify on vogix's OpenRGB build ([8ee91b7](https://github.com/i-am-logger/vogix/commit/8ee91b71eba799c5b8d8214e202c5a6630c98572))
* **reload:** user apply hooks under [hooks], and not-running apps are not failures ([d9c6760](https://github.com/i-am-logger/vogix/commit/d9c676028ecdeecbf8ff7f9b5f8fee31e02fcd3e))


### Bug Fixes

* **cli:** an undo or redo whose state is saved publishes it though its history write fails ([1826179](https://github.com/i-am-logger/vogix/commit/1826179cb26a8b1fa0f63228e57587aae468249b))
* **console:** one ANSI mapping for console.colors, theme packages and the runtime render ([87ea81d](https://github.com/i-am-logger/vogix/commit/87ea81df286afa71ba817ea49e84f4bd328f8a00))
* **desktop:** a hidden HUD samples nothing ([1f6f5c5](https://github.com/i-am-logger/vogix/commit/1f6f5c51d14ad5a8f4ed0656ab961e9df70cebe1))
* **desktop:** a reload applies a custom cell's new command and interval ([d5669d6](https://github.com/i-am-logger/vogix/commit/d5669d6c8e7a2c48bd179382762d29d0bcfe5512))
* **desktop:** a reload that changes the spectrum's band count restarts cava on it ([48d4e6f](https://github.com/i-am-logger/vogix/commit/48d4e6f56bd3c6f5aba8ab20cf436292932fafa6))
* **desktop:** a tap's source reads idle only once the tap has exited ([bcd6cce](https://github.com/i-am-logger/vogix/commit/bcd6cce6da71af8b1c0cd0ae445dc910c5f1ac44))
* **desktop:** kill an audio tap that has not exited 2 s after its stop ([420b7cf](https://github.com/i-am-logger/vogix/commit/420b7cf058c14de19288a03147147394e054d768))
* **desktop:** notification cards and panel popups keep clear of the bars ([da9d766](https://github.com/i-am-logger/vogix/commit/da9d76647462359e87f9ade86d51487c43c525fe))
* **desktop:** notification cards size their content from the card, not the FrameCell slot ([e18f386](https://github.com/i-am-logger/vogix/commit/e18f3863e9d2a9770e75de801328b3a74d896e01))
* **desktop:** per-mount I/O on LUKS and LVM, and total I/O on every disk ([5f53637](https://github.com/i-am-logger/vogix/commit/5f5363739b0e0cd90b2796bdbcaae16b63a98eb6))
* **desktop:** reattach to NetworkManager when it starts after the shell ([778a547](https://github.com/i-am-logger/vogix/commit/778a547532b94186b79ed26caa3384df8e2374e2))
* **desktop:** restored notification cards keep their arrival time ([2ff01a1](https://github.com/i-am-logger/vogix/commit/2ff01a1bc7c7697e83fbc01d7370f34ea5b819f1))
* **desktop:** scanlines texture the notification cards again; drop the primitives the HUD outgrew ([3ce9930](https://github.com/i-am-logger/vogix/commit/3ce993030ec02f070c51cd780674a22e31b8e7b3))
* **desktop:** stopping a custom cell's command ends every process of its pipeline ([39a3e48](https://github.com/i-am-logger/vogix/commit/39a3e48f3e05d9014ae60523d95086d4ef0e04d0))
* **desktop:** tailnet connection time is the connection's, and visible again ([feb271d](https://github.com/i-am-logger/vogix/commit/feb271d176b6c8245f3eaf823b9a60995fa612ab))
* **desktop:** the audio taps wait for PipeWire and come back after they exit ([08b1cf3](https://github.com/i-am-logger/vogix/commit/08b1cf3e47e8ba0952e96a90c0b84c1344cdc651))
* **desktop:** the GPU cell publishes the mean of its recent samples ([6724bfc](https://github.com/i-am-logger/vogix/commit/6724bfcf68dfe0990561db5053e5750ebd13ca6c))
* **desktop:** the GPU cell stays on when its bar comes back before nvidia-smi has exited ([713dabf](https://github.com/i-am-logger/vogix/commit/713dabff47129b20ae4fc6f924fb2f4d29aa334e))
* **desktop:** the LANG cell lights the layout Hyprland reports active, by index ([8e8cca9](https://github.com/i-am-logger/vogix/commit/8e8cca9e662f96f9ceb9b4d372eab53de55e7b26))
* **desktop:** the media transport stays on the player it paused ([7210c08](https://github.com/i-am-logger/vogix/commit/7210c089cbffb6cf12431d9f5da1f6d5f0844745))
* **desktop:** the mounts gauges lead with the root filesystem ([b492c14](https://github.com/i-am-logger/vogix/commit/b492c14e1427b1ad6bee815934c14c6a75c1dd8a))
* **desktop:** the oscilloscope sizes its canvas from its own scale ([e3eeda3](https://github.com/i-am-logger/vogix/commit/e3eeda34a315a21e5f6d7f4dd9c76d2614134ff4))
* **desktop:** the output VU reads the level applications send on every virtual sink ([8b736d5](https://github.com/i-am-logger/vogix/commit/8b736d5203f360a0f7b42dc3bf48fb03cb52bb35))
* **desktop:** the privacy cell shows live screencasts ([812547a](https://github.com/i-am-logger/vogix/commit/812547a5b5818b5f67de7b658cb93b4a377908b3))
* **desktop:** the Remind menu entry passes its delay as --in, and desktop check parses every menu command ([ef5ab09](https://github.com/i-am-logger/vogix/commit/ef5ab090d5529b98749d83d8fb823f5dcb85e63e))
* **desktop:** the session lock's locked state follows a lock that engages ([254d455](https://github.com/i-am-logger/vogix/commit/254d455f1af5e080193ca2e0f3191591c8af1a38))
* **desktop:** the shell runs without quickshell's detailed debug log ([64f9ad0](https://github.com/i-am-logger/vogix/commit/64f9ad0c939106ea67a93e8ec4b84ff005bf11e0))
* **desktop:** the shell's unit declares the tools it spawns, and the NixOS module enables UPower and power-profiles-daemon ([15b00dd](https://github.com/i-am-logger/vogix/commit/15b00ddcaa627b3f5c648ecae0d90e86db348baf))
* **desktop:** the spectrums keep their size while nothing plays ([fa9aa71](https://github.com/i-am-logger/vogix/commit/fa9aa710e104e6585fa9b0dca7db42cdd5d4705e))
* **desktop:** the tailnet glyph opens its panel beside its bar ([e98d9ac](https://github.com/i-am-logger/vogix/commit/e98d9ac4f9c06b80992f9cc25e7adb24bcfa67b9))
* **desktop:** the workspaces column and the LANG cell fit a rail ([b1ac4ad](https://github.com/i-am-logger/vogix/commit/b1ac4ad553fdbde9e751259318e5fe41f8dd57dd))
* **desktop:** tray menus anchor to their icon and open away from the bar ([d9c936b](https://github.com/i-am-logger/vogix/commit/d9c936bad200eccd6eddc410db7a5a47f7fab7c2))
* **desktop:** workspace clicks focus through the dialect-aware activate() ([1f16d77](https://github.com/i-am-logger/vogix/commit/1f16d770bb66b1dbe292da1b9f4cf004f24e463a))
* **home-manager:** restore the theme once per graphical session, not in every login shell ([85d7043](https://github.com/i-am-logger/vogix/commit/85d7043ab92a3934da7cc0ce1b838211bb65d4b0))
* **input:** Hyprland writes go out only in the dialect the compositor has stated ([d42f541](https://github.com/i-am-logger/vogix/commit/d42f541cbd80ca35a5571462b4d77df008d45699))
* **input:** the mode's border is painted again after a config reload or compositor restart ([a759741](https://github.com/i-am-logger/vogix/commit/a75974151652a099176cc5594531670615fcd038))
* **machine:** a command device ends on the published colour when the palette returns to it during a run ([2963290](https://github.com/i-am-logger/vogix/commit/2963290fb335625beb94b84e0a61dd9e870e21d3))
* **machine:** a palette rejection and status text print no control characters ([9a3d82c](https://github.com/i-am-logger/vogix/commit/9a3d82cd30cba155f3b0a4f236650090564663e8))
* **machine:** both owners end the same way when the drop zone goes, and both units restart alike ([d8eed46](https://github.com/i-am-logger/vogix/commit/d8eed469d93c578481fbb812af9147bcaca79a54))
* **nix:** a cross-built system validates machine.json with the build platform's vogix ([8b5c06b](https://github.com/i-am-logger/vogix/commit/8b5c06b7ed13e901fe01b9635d0056486bd6add4))
* **nix:** no user's theme apply writes the VT palette vogix-machine owns ([7c648b0](https://github.com/i-am-logger/vogix/commit/7c648b0cb9732257cc9198f09717085051db3696))
* **openrgb:** a controller whose target changed reports pending until it is applied ([b14b83c](https://github.com/i-am-logger/vogix/commit/b14b83c160a3fce9534ac365f598f5a079d25dfa))
* **openrgb:** the server keeps a client alive while anything still sends to it ([7e68b61](https://github.com/i-am-logger/vogix/commit/7e68b61f933d0949a0d5d92c63a9a7946285bd95))
* **state:** a user without state starts at their configured theme ([aedf7bb](https://github.com/i-am-logger/vogix/commit/aedf7bb8562c77071b5d3b7ea884c6509f185a76))


### Performance Improvements

* **desktop:** the meters cost nothing while nothing plays ([7d35641](https://github.com/i-am-logger/vogix/commit/7d356415caeb9b96f8e81ddd49b9bc355c8d6675))

## [0.11.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.10.0...vogix-v0.11.0) (2026-09-04)


### ⚠ BREAKING CHANGES

* **desktop:** four bars per monitor — desktop.json schema 2

### Features

* **desktop:** Flight Deck notification cards + tray and reminder parity ([e126dce](https://github.com/i-am-logger/vogix/commit/e126dce4b3cc0cfc29d513e6dc14ad45a6abf654))
* **desktop:** focus brackets — the Flight Deck corner marks on the focused window ([a1982ba](https://github.com/i-am-logger/vogix/commit/a1982ba09b7516e885a85e2a7bcadc17ec4db841))
* **desktop:** four bars per monitor — desktop.json schema 2 ([e0d0ad3](https://github.com/i-am-logger/vogix/commit/e0d0ad36852f753ba65281ade33ffc112673a67d))
* **desktop:** four bars per monitor, per-edge IPC, axis-aware widgets ([89e9bdc](https://github.com/i-am-logger/vogix/commit/89e9bdca3014d7e22eff8a2af9c85ca0a4be2f30))
* **desktop:** HUD feedback round — gauge cells, stereo VU, graphs beside their stats ([2ff83e7](https://github.com/i-am-logger/vogix/commit/2ff83e7b5243003823c93192761cc59c2fac0516))
* **desktop:** HUD type scale and component primitives ([697d4c2](https://github.com/i-am-logger/vogix/commit/697d4c2dd15d752ca57f5d80b6fc4835a478799b))
* **desktop:** no title readouts on the bars ([d210ef3](https://github.com/i-am-logger/vogix/commit/d210ef34811f7ed3400c692b8a2cebd3fafb3af0))
* **desktop:** opt-in CRT scanlines on the HUD chrome ([3b7d88b](https://github.com/i-am-logger/vogix/commit/3b7d88b21d3b206a0d6d3e1f36247539f7c69040))
* **desktop:** show CapsLock state in the LANG cell ([0eb5e3e](https://github.com/i-am-logger/vogix/commit/0eb5e3e7ebe58d65bb67d72688f0388ef8c97084))
* **desktop:** stereo corner spectrums and a real oscilloscope ([6951e65](https://github.com/i-am-logger/vogix/commit/6951e657d9b6f3051e46c0a181f7f821ca4f0405))
* **desktop:** the Flight Deck chrome pass — break-title cells everywhere, 10 Hz meters ([8d80723](https://github.com/i-am-logger/vogix/commit/8d807230f8776211410f78a574b6cd431d02ed0e))
* **desktop:** the HUD widget set — stat cells, VU meters, spectrum, graphs, rails ([684d4f1](https://github.com/i-am-logger/vogix/commit/684d4f19d10a46c128709bba189f5665999d39e8))
* **desktop:** the HUD's data services — VU peaks, spectrum, stats, layout, tailscale, privacy ([c78514f](https://github.com/i-am-logger/vogix/commit/c78514f0582b21662eb6e84e0fca9eeac5a24f88))
* **desktop:** the instrument rails — audio pickers, GPU/mounts/uptime cells ([8963f17](https://github.com/i-am-logger/vogix/commit/8963f170a1ae4ecdc5e131dd7bc5be52909b10fd))
* **input:** Alt+CapsLock cycles the layout, CapsLock still capitalises ([96984ab](https://github.com/i-am-logger/vogix/commit/96984ab5490a45ab0f6645b75e449cd1216eddfb))


### Bug Fixes

* **desktop:** a failed input.json load must clear the mode table ([cb0cf9e](https://github.com/i-am-logger/vogix/commit/cb0cf9e637d3a4ffb723c4d7371c7eb19bd5b556))
* **desktop:** active-window tracking without the missing toplevel protocol ([0edab88](https://github.com/i-am-logger/vogix/commit/0edab88eab862bb9c120796a730773c09ca4a843))
* **desktop:** the shell's own audio taps no longer light the mic privacy dot ([4e2df94](https://github.com/i-am-logger/vogix/commit/4e2df94ac2dc9d9099585f471d054e4c48e34467))
* **desktop:** the waveform service cannot be named Scope ([984646a](https://github.com/i-am-logger/vogix/commit/984646a48f37187d32828630c5abfaa6b82fb02c))
* **desktop:** whole-cell clicks live in FrameCell, not filling mouse areas ([d5cc7bf](https://github.com/i-am-logger/vogix/commit/d5cc7bf14b1fe97429dd52cd9a342468cdf35711))


### Performance Improvements

* **desktop:** VU ballistics tick at 30 fps, not the display's refresh rate ([00e3b4b](https://github.com/i-am-logger/vogix/commit/00e3b4b8527c049674211a150aaee038476db0db))

## [0.10.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.9.0...vogix-v0.10.0) (2026-08-29)


### Features

* **desktop:** the vogix desktop shell v1 — bar to boot splash ([#195](https://github.com/i-am-logger/vogix/issues/195)) ([4241dee](https://github.com/i-am-logger/vogix/commit/4241dee7a245e808b143bf619bda25742feffde5))

## [0.9.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.8.4...vogix-v0.9.0) (2026-08-28)


### Features

* **hypr:** Hyprland Lua config engine — dual-mode runtime, dialect-aware verbs, Lua generation ([#193](https://github.com/i-am-logger/vogix/issues/193)) ([4826759](https://github.com/i-am-logger/vogix/commit/482675928a5734f1c05a902a24a3f5ba025ae667))

## [0.8.4](https://github.com/i-am-logger/vogix/compare/vogix-v0.8.3...vogix-v0.8.4) (2026-08-16)


### Bug Fixes

* **wezterm:** let image-aware applications receive Ctrl+V ([#190](https://github.com/i-am-logger/vogix/issues/190)) ([5008eed](https://github.com/i-am-logger/vogix/commit/5008eeda33aafe6a68d58082ccab49b27844f81a))

## [0.8.3](https://github.com/i-am-logger/vogix/compare/vogix-v0.8.2...vogix-v0.8.3) (2026-08-10)


### Bug Fixes

* **nix:** compute the templates hash without import-from-derivation ([#186](https://github.com/i-am-logger/vogix/issues/186)) ([facd0ee](https://github.com/i-am-logger/vogix/commit/facd0ee1354b08a8f0ccae8906ccc464a36a1ddb))

## [0.8.2](https://github.com/i-am-logger/vogix/compare/vogix-v0.8.1...vogix-v0.8.2) (2026-07-01)


### Bug Fixes

* **input:** release held modifiers when a keyboard disconnects ([#184](https://github.com/i-am-logger/vogix/issues/184)) ([9db77f2](https://github.com/i-am-logger/vogix/commit/9db77f2117da1fbe7463b7f3d082cb2eb1fde830))

## [0.8.1](https://github.com/i-am-logger/vogix/compare/vogix-v0.8.0...vogix-v0.8.1) (2026-06-19)


### Bug Fixes

* **deps:** adopt praxis 0.25.4 — source WM-nav presets from praxis ([#181](https://github.com/i-am-logger/vogix/issues/181)) ([bc357dd](https://github.com/i-am-logger/vogix/commit/bc357dd21b65546f6e5adca602f8cee4c506754f))

## [0.8.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.7.1...vogix-v0.8.0) (2026-06-17)


### Features

* variant-nav ramp-stepping + illumination-preserving theme switches (+ docs↔code audit) ([#177](https://github.com/i-am-logger/vogix/issues/177)) ([60fbe52](https://github.com/i-am-logger/vogix/commit/60fbe5264b39ccb925e2e80d97ed06030b5863be))

## [0.7.1](https://github.com/i-am-logger/vogix/compare/vogix-v0.7.0...vogix-v0.7.1) (2026-06-17)


### Bug Fixes

* **deps:** adopt vogix16-themes v0.2.0 ([#175](https://github.com/i-am-logger/vogix/issues/175)) ([a739734](https://github.com/i-am-logger/vogix/commit/a739734fc4cacedc438a1c5c6e19e4effeb98fd3))

## [0.7.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.4...vogix-v0.7.0) (2026-06-16)


### Features

* appearance, hardware, and console modules ([#169](https://github.com/i-am-logger/vogix/issues/169)) ([64ffb34](https://github.com/i-am-logger/vogix/commit/64ffb3458bf8c20d3297fc07420744b700e42c48))
* F12 console mode + tmux + semantic colors + shader prep ([a8266e2](https://github.com/i-am-logger/vogix/commit/a8266e2a0b6a2859b2c08ccfa6a85eeb8dbc1dff))
* **input:** engine-resolved multi-paradigm keybindings + praxis 0.25.0 ([#172](https://github.com/i-am-logger/vogix/issues/172)) ([d764b68](https://github.com/i-am-logger/vogix/commit/d764b680ce1cf6ab8db652a3c9b66ff8ca3a4db0))
* monochromatic shader + app theming (bespec, wezterm, btop) ([80dcfbe](https://github.com/i-am-logger/vogix/commit/80dcfbe05c7a87fa89ea1c93df60d62c0f735d4b))
* refactor CLI + session/daemon with undo stack ([3f15cf2](https://github.com/i-am-logger/vogix/commit/3f15cf248e6d252c936e29a5791a579137a2fa42))
* shader on accepts -i/-b/-s params for intensity/brightness/saturation ([1e942ef](https://github.com/i-am-logger/vogix/commit/1e942effcf0d78e10b96724cd708aeb67956841d))
* shader on/off/toggle CLI + status display ([f229885](https://github.com/i-am-logger/vogix/commit/f22988522a07f8380fd7e82146ac812d571d72a7))


### Bug Fixes

* default shader intensity to 0.7 ([8b2cc01](https://github.com/i-am-logger/vogix/commit/8b2cc01c80b552667f4631efae0bf18bd1274b4d))

## [0.6.4](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.3...vogix-v0.6.4) (2026-03-18)


### Bug Fixes

* use crate2nix fork without crate2nix_stable input ([#165](https://github.com/i-am-logger/vogix/issues/165)) ([58b3561](https://github.com/i-am-logger/vogix/commit/58b3561e518c14bf2aad629b161091e49c0672f6))

## [0.6.3](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.2...vogix-v0.6.3) (2026-03-18)


### Bug Fixes

* deduplicate all flake.lock inputs and fix theme import ([#163](https://github.com/i-am-logger/vogix/issues/163)) ([0ae2f7f](https://github.com/i-am-logger/vogix/commit/0ae2f7f59ad749e6db3bcf9aa68751c2d24b4f1a))

## [0.6.2](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.1...vogix-v0.6.2) (2026-03-17)


### Bug Fixes

* deduplicate nixpkgs in flake.lock via follows ([#161](https://github.com/i-am-logger/vogix/issues/161)) ([24ad2a2](https://github.com/i-am-logger/vogix/commit/24ad2a2e8e33a21887207612bca18404445fd29a))

## [0.6.1](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.0...vogix-v0.6.1) (2026-01-25)


### Bug Fixes

* **nix:** simplify option names and add reusable overlay ([20bffa1](https://github.com/i-am-logger/vogix/commit/20bffa13731e4ef74566265980d1964dc42af627))

## [0.6.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.5.1...vogix-v0.6.0) (2026-01-20)


### Features

* **architecture:** implement template-based theme system ([#140](https://github.com/i-am-logger/vogix/issues/140)) ([9de7493](https://github.com/i-am-logger/vogix/commit/9de749349c39010d4ebd297990ff3ef4c1e32162))

## [0.5.1](https://github.com/i-am-logger/vogix/compare/vogix-v0.5.0...vogix-v0.5.1) (2026-01-19)


### Bug Fixes

* **deps:** update flake inputs ([e13f045](https://github.com/i-am-logger/vogix/commit/e13f045c08271a2790cb6eeeed02466c225609cb))

## [0.5.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.4.0...vogix-v0.5.0) (2026-01-19)


### Features

* add multi-scheme support for base16, base24, ansi16 ([122354b](https://github.com/i-am-logger/vogix/commit/122354bcf38b1a52ff0e335349d0812b12dc58e0))
* **app:** add ripgrep theme support ([#112](https://github.com/i-am-logger/vogix/issues/112)) ([1451178](https://github.com/i-am-logger/vogix/commit/145117824302d89c694424a1dd8072856aa449c7))
* **devenv:** add treefmt-nix for unified formatting ([#126](https://github.com/i-am-logger/vogix/issues/126)) ([574951a](https://github.com/i-am-logger/vogix/commit/574951a0fa9f8ee661629141ac4557c8ba834aaf)), closes [#125](https://github.com/i-am-logger/vogix/issues/125)
* integrate devenv with flake and optimize CI workflow ([#110](https://github.com/i-am-logger/vogix/issues/110)) ([b3480aa](https://github.com/i-am-logger/vogix/commit/b3480aa7dff2f1cad54f8d123498bb3b5d21d491))
* integrate with home-manager settings system ([#115](https://github.com/i-am-logger/vogix/issues/115)) ([45c9c96](https://github.com/i-am-logger/vogix/commit/45c9c96ffbc257a6b02bf1ed03ae9985a5719982)), closes [#114](https://github.com/i-am-logger/vogix/issues/114)
* vogix16 runtime theme management for NixOS ([584c9f3](https://github.com/i-am-logger/vogix/commit/584c9f3ddbb519fd6869e3e4259b7819da2028c1))


### Bug Fixes

* **ci:** upgrade cache-nix-action to v7 and add release-please manifest ([3ee3adf](https://github.com/i-am-logger/vogix/commit/3ee3adf546d65c40b6d1afa67a12b25cb43150a2))
* correct version bump - templates removal is not breaking ([5960fe0](https://github.com/i-am-logger/vogix/commit/5960fe021f46a66b2909570852dad2084ac87b55))
* make VM tests generic and auto-discovering ([#108](https://github.com/i-am-logger/vogix/issues/108)) ([7138374](https://github.com/i-am-logger/vogix/commit/7138374c28a4e52a082e3a8f87cc12ae17447f17))
* **reload:** remove shell command injection in touch reload ([#122](https://github.com/i-am-logger/vogix/issues/122)) ([fd961b7](https://github.com/i-am-logger/vogix/commit/fd961b70b0bb56789092360905a23abc000f367f))
* resolve theme loading issues and improve development workflow ([946e62b](https://github.com/i-am-logger/vogix/commit/946e62b626b9465d128030dd8bbd79436a036021))
* resolve theme loading issues and improve development workflow ([da38cad](https://github.com/i-am-logger/vogix/commit/da38cad5b974da8ebfc84b9cb8583dc6dc4c1652))
* test searched for vogix16 bin rather then vogix ([69c488c](https://github.com/i-am-logger/vogix/commit/69c488c0c570bb778cb0027e8763a283a4d48ecf))

## [0.4.0](https://github.com/i-am-logger/vogix/compare/v0.3.1...v0.4.0) (2026-01-17)


### Features

* **devenv:** add treefmt-nix for unified formatting ([#126](https://github.com/i-am-logger/vogix/issues/126)) ([574951a](https://github.com/i-am-logger/vogix/commit/574951a0fa9f8ee661629141ac4557c8ba834aaf)), closes [#125](https://github.com/i-am-logger/vogix/issues/125)

## [0.3.1](https://github.com/i-am-logger/vogix/compare/v0.3.0...v0.3.1) (2026-01-15)


### Bug Fixes

* **reload:** remove shell command injection in touch reload ([#122](https://github.com/i-am-logger/vogix/issues/122)) ([fd961b7](https://github.com/i-am-logger/vogix/commit/fd961b70b0bb56789092360905a23abc000f367f))

## [0.3.0](https://github.com/i-am-logger/vogix/compare/v0.2.0...v0.3.0) (2025-12-03)


### Features

* **app:** add ripgrep theme support ([#112](https://github.com/i-am-logger/vogix/issues/112)) ([1451178](https://github.com/i-am-logger/vogix/commit/145117824302d89c694424a1dd8072856aa449c7))

## [0.2.0](https://github.com/i-am-logger/vogix/compare/v0.1.3...v0.2.0) (2025-12-03)


### Features

* integrate devenv with flake and optimize CI workflow ([#110](https://github.com/i-am-logger/vogix/issues/110)) ([b3480aa](https://github.com/i-am-logger/vogix/commit/b3480aa7dff2f1cad54f8d123498bb3b5d21d491))

## [0.1.3](https://github.com/i-am-logger/vogix/compare/v0.1.2...v0.1.3) (2025-12-03)


### Bug Fixes

* make VM tests generic and auto-discovering ([#108](https://github.com/i-am-logger/vogix/issues/108)) ([7138374](https://github.com/i-am-logger/vogix/commit/7138374c28a4e52a082e3a8f87cc12ae17447f17))

## [0.1.2](https://github.com/i-am-logger/vogix/compare/v0.1.1...v0.1.2) (2025-12-03)


### Bug Fixes

* correct version bump - templates removal is not breaking ([5960fe0](https://github.com/i-am-logger/vogix/commit/5960fe021f46a66b2909570852dad2084ac87b55))
* resolve theme loading issues and improve development workflow ([946e62b](https://github.com/i-am-logger/vogix/commit/946e62b626b9465d128030dd8bbd79436a036021))
* resolve theme loading issues and improve development workflow ([da38cad](https://github.com/i-am-logger/vogix/commit/da38cad5b974da8ebfc84b9cb8583dc6dc4c1652))

## [0.1.1](https://github.com/i-am-logger/vogix/compare/v0.1.0...v0.1.1) (2025-12-02)


### Bug Fixes

* test searched for vogix16 bin rather then vogix ([69c488c](https://github.com/i-am-logger/vogix/commit/69c488c0c570bb778cb0027e8763a283a4d48ecf))

## 0.1.0 (2025-12-02)


### Features

* vogix16 runtime theme management for NixOS ([584c9f3](https://github.com/i-am-logger/vogix/commit/584c9f3ddbb519fd6869e3e4259b7819da2028c1))

## [Unreleased]

### Added
- GitHub Actions CI/CD workflows
- Comprehensive CONTRIBUTING.md with development guidelines
- Automated release process with release-please

### Changed
- Updated all documentation to reflect actual Nix-based architecture
- Corrected README examples to show proper home-manager integration
- Binary name consistently referred to as `vogix` throughout docs

### Removed
- Obsolete ARCHITECTURE-REDESIGN.md documentation

## [0.5.0] - 2024-XX-XX

### Added
- Renamed binary from `vogix16` to `vogix` for consistency
- Auto-toggle `switch` command (no arguments needed)
- Per-app theme and variant override support
- Comprehensive theme library with 19 themes
- Console (TTY) theme support with setvtrgb integration

### Changed
- Refactored architecture: Nix generates all theme configs at build time
- CLI now only manages symlinks, not config generation
- Improved symlink architecture for instant theme switching
- Enhanced state persistence

## [0.4.0] - 2024-XX-XX

### Added
- Comprehensive automated testing suite (16 test scenarios)
- VM-based integration testing
- Theme validation at build time
- Semantic color API for application modules

### Changed
- Improved error handling across all components
- Enhanced reload mechanisms with multiple methods
- Better symlink management

## [0.3.0] - 2024-XX-XX

### Added
- Auto-discovery of themes from `themes/` directory
- Auto-discovery of application generators
- Systemd service for runtime directory setup
- Support for per-app configuration overrides

### Changed
- Migrated from runtime template processing to build-time generation
- Simplified CLI to focus on symlink management

## [0.2.0] - 2024-XX-XX

### Added
- Multiple application reload methods (touch, signal, command)
- Shell completions for all major shells
- State persistence for current theme/variant

### Changed
- Improved NixOS integration
- Enhanced home-manager module

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- vogix16 design system with 16-color palette
- Basic theme switching functionality
- Dark and light variant support
- NixOS and home-manager integration
- Example themes (yoga, forest, matrix)
- CLI tool for runtime theme management
- Application reload mechanism
- Documentation for design system, architecture, and usage

[Unreleased]: https://github.com/i-am-logger/vogix/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.5.0
[0.4.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.4.0
[0.3.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.3.0
[0.2.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.2.0
[0.1.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.1.0
