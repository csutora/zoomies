# zoomies

momentum scrolling for touchpads on wlroots-based wayland compositors. very tuneable!\
made this because other projects i've tried inject wheel-scroll events rather than finger-scroll ones which a lot of apps don't seem to handle well.

## compatibility

zoomies injects scroll through `zwlr_virtual_pointer`, a wlroots protocol. so it works on wlroots-based compositors:

- hyprland (tested)
- sway, river, wayfire, labwc, and other wlroots compositors (expected, not yet confirmed)

it does **not** work on gnome (mutter) or kde (kwin), which don't implement that protocol. (feel free to open a pull request!)

## install

### nix (flakes)

add zoomies to your flake inputs:

```nix
{
    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

        zoomies = {
            url = "github:csutora/zoomies";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };

    outputs = { self, nixpkgs, zoomies, ... }: {
        nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
            modules = [
                zoomies.nixosModules.default
                {
                    services.zoomies.enable = true;
                }
            ];
        };
    };
}
```

the module runs zoomies as a user service in your graphical session.

your user needs read access to the touchpad's `/dev/input/eventN`. on most distros that means being in the `input` group:

```nix
users.users.your-username.extraGroups = [ "input" ];
```

then relog (or reboot) for the group to take effect.

### manual

```bash
git clone https://github.com/csutora/zoomies
cd zoomies
cargo build --release
./target/release/zoomies --help
```

run it inside your wayland session (it needs `WAYLAND_DISPLAY`), and make sure you can read the touchpad's `/dev/input/eventN` (be in the `input` group, or grant access some other way). then launch it from your compositor's autostart or a user service.

## configuration

every option, with its default:

```nix
services.zoomies = {
    enable = true;

    deviceName = "Magic Trackpad"; # substring of the touchpad's name to match
    multiplier = 0.1;              # scroll distance per flick
    decayMs = 325.0;               # glide time
    minVelocity = 200.0;           # how hard a flick must be to start momentum
    stopThreshold = 40.0;          # speed at which the glide stops
    tickMs = 8;                    # ms between emitted scroll frames
    naturalScroll = true;          # follow natural scroll direction
};
```

the same knobs are available as flags for the manual setup, see `zoomies --help`.

if momentum feels off, `multiplier` (distance) and `decayMs` (length) are the first things you likely want to experiment with.

## license

mit
