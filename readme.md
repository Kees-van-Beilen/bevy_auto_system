# Bevy Auto System
This crate is designed to help with the quick creation of systems when time isn't on your side and it doesn't even screw up the type hints. Please note that this crate is mega experimental and not yet in a finished state. 

- [Bevy Auto System](#bevy-auto-system)
  - [prelude](#prelude)
  - [macros](#macros)
    - [`query![]`](#query)
    - [`spawn!(x)`](#spawnx)
    - [`load!(x)`](#loadx)
    - [`time!()`](#time)
    - [`delta_time!()` and `delta_seconds!()`](#delta_time-and-delta_seconds)
    - [`windows!()`](#windows)
    - [`resource!(x)` and `res!(x)`](#resourcex-and-resx)
    - [`resource!(mut x)` and `res!(mut x)`](#resourcemut-x-and-resmut-x)


## prelude
- add to your cargo.toml with 
```
bevy_auto_sys = {git = "https://github.com/Kees-van-Beilen/bevy_auto_system"}
```
- import everything from the crate `use bevy_auto_sys::*`
- Every function that utilises the auto-sys macros must have a `#[auto_system]` tag. *note: your IDE will put some red lines under it that's because auto_system is locked behind a feature that the language server doesn't thus ensuring correct type completions.*
- when compiling make sure to add auto-sys feature via the command line like so: 
```bash
cargo run --features bevy_auto_sys/auto-sys
```
- now your all set to use auto systems

## macros
### `query![]`
smartly construct a query based on where and how it's used. Below are a few scenarios:
|context|param|context out|
|-|-|-|
|query![Transform]|query_transform:Query<&Transform>|query_transform|
|for transform in query![Transform]|query_transform:Query<&Transform>|query_transform.iter()|
|for mut transform in query![Transform]|mut query_transform:Query<&mut Transform>|query_transform.iter_mut()|

Currently the query syntax is very basic only supporting the `With<T>` filter. The table below outlines the syntax

|context|query|
|-|-|
|query![Transform]|Query<&Transform>|
|query![Transform Sprite]|Query<(&Transform,&Sprite)>|
|query![Transform and Sprite]|Query<(&Transform,&Sprite)>|
|query![Transform, Sprite]|ERROR|
|query![Transform with Sprite]|Query<&Transform,With<Sprite>>|
|query![Transform Texture with Sprite]|Query<(&Transform,&Texture),With<Sprite>>|
|query![Transform Texture with Sprite and Visibility]|Query<(&Transform,&Texture),With<(Sprite,Visibility)>>|
|query![Transform Texture with Sprite with Visibility]|Query<(&Transform,&Texture),With<(Sprite,Visibility)>>|
|query![Transform Texture with Sprite Visibility]|Error|
### `spawn!(x)`
shorthand for `commands.spawn(x)`, also imports `mut command:Commands`. Currently there is a type completion problem inside of the spawn! macro
### `load!(x)`
shorthand for `asset.load(x)`, also imports `assets:Res<Assetserver>`
### `time!()`
shorthand for `time`, also imports `time:Res<Time>`
### `delta_time!()` and `delta_seconds!()`
shorthand for `time.delta_seconds()`, also imports `time:Res<Time>`
### `windows!()`
shorthand for `windows`, also imports `windows:Res<Windows>`
### `resource!(x)` and `res!(x)`
shorthand for `resource_x`, also imports `resource_x:Res<X>`
### `resource!(mut x)` and `res!(mut x)`
shorthand for `resource_x`, also imports `mut resource_x:ResMut<X>`