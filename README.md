# yume
Yume Nikki fangame with Rust + SDL2

# Table of Contents
- [Tiled Properties](#tiled-properties)
- [Actions](#actions)
- [Animation](#animation)
- [AI](#ai)
- [Particles](#particles)
- [Transitions](#transitions)
- [Properties](#properties)
- [Conditions](#conditions)

# Tiled Properties
- [Map](#map)
- [Layer](#layer)
- [Entity](#entity)
- [Tile](#tile)
## Map
- **clamp_camera (legacy clampCamera) (bool):**
Whether or not the clamp the camera to the edges of the map
- **clamp_camera_axis (string):**
What axes to use for clamping the camera (`"x"`, `"y"`, `"all"`)
- **default_pos (legacy defaultPos) (string):**
Default position for the player, used in debug warp, default warp coordinate. `"x,y"`
- **edges (JSON):**
Define an [Action](#actions) for when the player reaches each edge of the map (`"up"`, `"down"`, `"left"`, `"right"`)
```
{
	"down": {
		"type": "warp",
		"map": "hospital/hospital_entrance.tmx",
		"pos": { "x": 8, "y": 5 },
		"transition": "fade"
	}
}
```
- **looping (bool):**
Whether or not the map should loop
- **looping_axis (string):**
Which axes the map should loop on (`"x"`, `"y"`, `"all"`)
- **music (string):**
Full path starting from res/ to the music to play in the world, ex. `"res/audio/music/animated0.ogg"`
- **music_speed (float):**
Speed of the song to be played, `1.0` being the default
- **music_volume (float):**
Volume to play the song at, `1.0` being the default
- **tint (string):**
The map's tint color (`"r,g,b,a"`)
- **raindrops (bool):**
Whether or not to create raindrop particles on applicable tiles (you have to do the foreground effect with an image layer)
- **snow (bool):**
Whether or not to draw the foreground snow effect 

## Layer
- **height (int):**
Height of this layer. Drawn layers are sorted by height, then ordering in the .tmx file
- **draw (bool):**
Whether or not this layer is rendered
- **collide (bool):**
Whether or not this layer has collisions enabled
- **name (string):**
Name of this layer for use in `World::get_mut_layer_by_name` TODO ?????

### Image layers
- **looping (bool):**
Whether or not the image layer is drawn looping
- **looping_x (bool):**
Enable looping only on the X axis
- **looping_y (bool):**
Enable looping only on the Y axis
- **scroll_x (int):**
Amount to scroll this layer in the X axis, in pixels per frame
- **scroll_y (int):**
Amount to scroll this layer in the Y axis
- **x (int):**
Starting X position
- **y (int):**
Starting Y position
- **delay_x (int):**
Delay between image scrolling on X axis, in frames, for very slow movement
- **delay_y (int):**
Delay between image scrolling on Y axis
- **mismatch (bool):**
If `mismatch` is true, the image layer will be move along each axis on alternating frames
- **parallax_x (int):**
Amount of parallax effect on X axis, higher values appear farther away
- **parallax_y (int):**
Amount of parallax effect on Y axis
- **height (int):**
Same effect as on a regular layer

## Entity
- **height (int):**
Height (layer) this entity is drawn at (player is at 0 by default)
- **solid (bool):**
Whether or not collision is enabled for this entity
- **draw (bool):**
Whether or not this entity is drawn to the screen (useful for event controllers)
- **walk_behind (bool):**
If this is true, when the player is above the object it will be draw above the player (ex. doors)
- **collider (JSON):**
JSON object with `x`, `y`, `w`, `h` properties
```
{
	"x": 0,
	"y": 16,
	"w": 16,
	"h": 16
}
```
- **ai (JSON):**
See [AI](#ai)
- **animation (JSON):**
See [Animation](#animation)
- **actions (JSON):**
See [Actions](#actions)
- **particles (JSON):**
See [Particles](#particles)
- **file (string):**
Path to a JSON file to load as properties onto this entity<br>
A string beginning with a $ will be replaced with the according tiled property
ex: `"x": "$warp_x"` would be replaced by the property `warp_x` in the entity's tiled properties

## Tile
- **blocking (bool):**
Whether or not this tile has collisions. Defaults to `false`
- **step (string):**
Step sounds for this tile. Sound effect names do not need file extensions
- **step_volume (float):**
Volume to play the step sound at. Defaults to `1.0`
- **stairs (bool):**
Whether or not this tile acts as stairs. Defaults to `false`
- **no_rain (bool):**
If true, raindrops will not fall on this tile. Defaults to `false`
- **speed_mod (int):**
Modifies player speed while standing on this tile. Changes the speed by `2^n`. Defaults to `0`
- **ladder (bool):**
Whether this tile acts as a ladder. Defaults to `false`

# AI
An AI object contains data on how an entity moves
```
"ai": {
  "type": "chaser",
  "speed": "$chaser_speed", 
  "detection_radius": 100
},
```
All AI objects start with a `type`

### **AI Type: `wander`**
The entity wanders around randomly
- **frequency (i32):**
How often the entity wanders, one out of `frequency`. Default is `100` (frames)
- **delay (i32):**
Minimum time between entity movements. Default is `25` (frames)
- **speed (u32):**
Entity speed (pixels per frame)
- **move_delay (u32):**
Delay between each movement frame, used to make very slow entities

### **AI Type: `chaser`**
The entity chases the player<br>
When the entity bumps into the player, a player bump interaction is simulated
- **speed (u32):**
Entity speed. The player's default speed is `1`, `2` with running shoes. Defaults to `1`
- **path_max (u32):**
For the `astar` pathfinder, the maximum steps the algorithm can make before giving up. Defaults to `10000`
- **detection_radius (u32):**
The maximum (manhattan) distance the chaser can chase after the player from. Defaults to `16`
- **pathfinder (string):**
One of `astar` or `walk_towards`. `astar` is very slow but traverses complicated maps. `walk_towards` just walks towards the player, stopping on walls

### **AI Type: `push`**
The entity is pushed on interaction
- **speed (u32):**
Entity speed. Defaults to `2`

### **AI Type: `animate_on_interact`**
The entity will play an animation on interaction. Useful for things like doors
- **frames (u32):**
Number of frames to advance on interaction. Defaults to `1`
- **use (bool):**
Can be triggered upon use. Ex: doors. Defaults to `false`
- **bump (bool):**
Can be triggered on bump. Defaults to `false`
- **walk (bool):**
Can be triggered on walk. Defaults to `false`
- **side (string) (optional):**
If present, only a certain side will trigger the animation.

## Animation
An animation is similar to an [AI](#ai) definition
```
"animation": {
  "type": "directional",
  "up": 3,
  "down": 1,
  "left": 2,
  "right": 0,
  "frames": 3,
  "speed": 4,
  "repeat": "cycle",
  "idle": 1
},
```
Every animation includes a `repeat`, defining how frames advance at loop boundaries
- **cycle:**
Animation cycles back and forth, like 0, 1, 2, 1, 0, 1, 2, ..., like the player
- **loop:**
Animation loops, like 0, 1, 2, 0, 1, 2, 0, 1, ...

Every animation also includes a `type`
### Animation type: **`still`**
Animation does not play
- **frame (u32) its the frame it sits still on**
### Animation type: **`sequence`**
Moves through a sequence of frames in order of the spritesheet
- **start (u32):**
Starting frame for the animation
- **length (u32):**
Total length of the animation in frames
- **speed (u32):**
Speed the animation is played at, in frames per frame advancement
- **idle (u32):**
Legacy property. Does nothing

### Animation type: **`directional`**
- **up (u32):**
Upwards facing row on the spritesheet
- **down (u32):**
Downwards facing row on the spritesheet
- **left (u32):**
Left facing row on the spritesheet
- **right (u32):**
Right facing row on the spritesheet
- **frames (u32):**
Amount of frames per row
- **speed (u32):**
Speed at which animation is played

### Animation type: **`follow`**:
Animation follows the player, like the eyes in paranoia world
- **center (u32):**
Center frame for the follow animation
- **axes (string):**
Axis in which the animation follows the player (`"all"`, `"x"` ,`"y"`)
- **speed (u32):**
Speed of the animation

Animations also can include
- **on_move (bool):**
Whether the animation only runs while the entity is moving
- **manual (bool):**
Whether the animation can only be triggered manually (ex. through the `animate_on_interact` AI type)<br>
If neither are set, the animation will always run

## Actions
All begin with a `type`<br>
And include a [Trigger](#triggers)
### **Action: `warp`**
Warp the player
- **map (string) (optional):**
Path starting from `res/maps/` to the map to warp to. If not specified, warp the player to the same map they are in
- **transition (JSON):**
See [Transitions](#transitions)
- **pos (JSON):**
Position of the warp Internally, `WarpPos`, formatted as `"pos": { "x": 5, "y": 5 }`<br>
X and Y can be numbers, or several keywords. `match` keeps the player's position for one component, 
`default` puts the player at the map's default position.<br>
X and Y can also be [IntProperty](#intproperty)

### **Action: `print`**
Print a message to the console
- **message (string)**

### **Action: `delayed`**
Delays an action for some amount of frames
- **delay (u32):**
Number of frames to wait until the action is triggered
- **after (JSON):**
Action to trigger after the delay

### **Action: `freeze`**
Freeze the player
- **time (u32) (optional):**
Time to freeze the player for (frames). If not present, toggle freeze on

### **Action: `give_effect`**
Give an effect to the player
- **effect (string):**
Effect to give to the player

### **Action: `set_flag`**
Set a flag within the game state
- **global (bool):**
Whether or not the flag is global. Global flags are not erased on level change. Defaults to `false`
- **flag (string):**
Name of desired flag to set
- **val (int):**
Value to set the flag to

### **Action: `conditional`**
Run an action based on a condition
- **condition (JSON):**
See [Conditions](#conditions)
- **action (JSON):**
Action to run if the condition is met

### **Action: `play`**
Play a sound effect
- **sound (string)**
Name of the sound effect to be played
- **speed (float)**
Speed at which to play the sound effect
- **volume (float)**
Volume at which to play the sound effect

### **Action: `set`**
Set a property within the game state
- **in (string)**
Category the game property lies within. Can be one of `player`, `world`, `entity`. Entity set commands must be called by the entity itself
- **val (string)**
Name of the value to be set within the category defined by `in`. See [Properties](#properties)
- **to (any):**
Value to set the property to

### **Action: `change_song`**
Change the current map's song
- **volume (float)**
Volume for the new song to be played at
- **speed (float)**
Speed for the new song to be played at
- **song (string)**
Path to the new song to be played (from root)
- **set_defaults (bool)**
If true, the song's default speed and volume will be overwritten

### **Action: `set_animation_frame`**
Set the animation frame of an entity. Works best with the animation in `manual` mode
- **target (string)**
Currently only `this` is supported
- **val (IntProperty)**
Frame id to set

### **Action: `multiple`**
Usually this isn't explicitly used, as an array of actions is parsed as a `multiple` action
- **actions (list of actions)**
Actions to run

### **Action: `set_variable` or `set_var`**
Sets a variable, which is local to the entity and can be any property type. This action is only valid when called by an entity.
- **store (bool)**
If `store` is true, the value passed into `val` is evaluated on the spot and the result is stored in the variable. Otherwise, the variable value will change if `val` changes.
- **var_type (string)**
One of `int`, `float`, `bool` (`boolean`), `string`
- **val (any)**
Value ([property](#properties)) that the variable is set to

### **Action: `sit`**
Makes the player move up one tile, negate effects, and enter a sitting state.

### **Action: `lay_down`**
Makes the player travel 3/2 of a tile in either facing horizontal direction, negate effects, and enter a lying down state. 

### **Action: `remove`**
Remove an entity
- **target (string | IntProperty)**
Either `self` (`this`) or an IntProperty corresponding to the tiled ID of the entity to remove

### **Action: `lay_down_in_place`**
Makes the player lay down without movement. Used upon waking up.
- **exit_dir (string)**
One of `up`, `down`, `left`, `right`. The direction the player exits the lying down state from. The player will travel 3/2 of a tile in this direction.
- **offset_x (IntProperty)**
X offset applied to lying state
- **offset_y (IntProperty)**
Y offset applied to lying state

### **Action: `move_player`**
- **direction (string)**
One of `up`, `down`, `left`, `right`
- **forced (bool)**
If true, the player will ignore all checks while moving
- **custom_distance (int)**
Can be used to change the distance moved to something aside from 16

## Triggers
Triggers begin with a `type`
- `use`: Triggered upon interaction. A `side` argument can be included
- `walk`: Triggered on being walked on. I dont know if `side` works with this one
- `bump`: Triggered on being bumped. A `side` argument can be included
- `interact`: Triggered on any interaction. Probably can use `side`
- `onload`: Triggered on level load
- `switch`: Triggered on effect switch
- `tick`: Triggered every `freq` (u32) game ticks (60fps)

# Particles
Particle emitters are included as an argument in an entity. Particle colors are based on textures.
## Particle types
Most properties in the particle emitter can exist as fixed values or ranges, which are chosen from randomly.<br>
Coordinate pairs and ranges are expressed with arrays. Ex: `"velocity": [[-1, 1], [-4, -2]]`<br>
Each property value has a reasonable default.
- **lifetime (u32, range):**
How long the particle lasts, in frames
- **pos_offset ((f32, f32), range)**
Offset from the entity origin that the particles are created from
- **velocity ((f32, f32), range)**
Initial velocity of the particles
- **acceleration ((f32, f32), range)**
Acceleration of the particles
- **tx_coord ((f32, f32), range)**
Starting texture coordinate for the particle
- **tx_vel ((f32, f32), range)**
Velocity of the texture coordinate for the particle
- **freq (u32, range)**
Frequency of emission of particles
- **texture_path (string)**
Path to the particle texture, local to the particle textures folder
- **size ((u32, u32))**
Size of the particle
- **freq_rand (i32)**
Random variation of the emitter frequency
- **stagnate (f32, range)**
The particle velocity will be divided by this every frame if set

# Transitions
Transition begins with a `type`. Some types require extra information
- `fade`
- `fade_to_color`: `r`, `g`, and `b` (u32) define the color
- `music_only`
- `spotlight`
- `spin`
- `zoom`: `scale` (f32) is the scale to zoom in
- `pixelate`:
- `lines`: `height` (u32) is the height of each line
- `wave`: `dir` (string) can be any of `horizontal`, `vertical`. Direction of the wave
- `grid_cycle`
- `player_fall`
Transitions also can include
- `speed` (int): Speed at which the transition is played. Defaults to `8`
- `music` (bool): Whether or not to fade the music with the transition
- `hold` (int): Number of frames to hold at the fully transitioned state
- `reset_music` (bool): Whether to reset the music to the beginning if the transition is between two maps with the same song

# Properties
A property is a value of type `int`, `float`, `string`, or `bool` that can be used within most fields and gets a value based on the game state or performs a calculation.
## IntProperty
An integer literal will be parsed as an `int` type IntProperty
- **`int`**: Contains a `val` with the integer value
- **`player`**: Contains a `property` corresponding to a [player property](#player-properties)
- **`entity`**: Contains a `property` corresponding to an [entity property](#entity-properties)
- **`level`**: Contains a `property` corresponding to a [level property](#level-properties)
- **`flag`**: Contains `global` and `flag` defining what flag to get
- **`add`**: Add `lhs` and `rhs`
- **`sub`**: Subtract `rhs` from `lhs`
- **`mul`**: Multiply `lhs` and `rhs`
- **`div`**: (Integer) divide `lhs` and `rhs`
- **`var` (`variable`)**: Get a variable `name` (StringProperty) from the calling entity

## FloatProperty
A float literal will be parsed as a `float` type FloatProperty
- **`float`**: Contains a `val` with the float value
- **`player`**: Contains a `property` corresponding to a [player property](#player-properties)
- **`level`**: Contains a `property` corresponding to a [level property](#level-properties)
- Note: There are not yet any float properties made available from entities
- **`add`**: Add `lhs` and `rhs`
- **`sub`**: Subtract `rhs` from `lhs`
- **`mul`**: Multiply `lhs` and `rhs`
- **`div`**: (Integer) divide `lhs` and `rhs`
- **`var` (`variable`)**: Get a variable `name` (StringProperty) from the calling entity

## StringProperty
A string literal will be parsed as a `string` type StringProperty
- **`string`**: Contains a `val` with the string value
- **`from_int`**: Convert an IntProperty `val` into a string
- **`concatenate`**: Concatenate two StringProperties `lhs` and `rhs` together
- **`var` (`variable`)**: Get a variable `name` (StringProperty) from the calling entity

## BoolProperty
A bool literal will be parsed as a `bool` type BoolProperty
- **`bool`**: Contains a `val` with the bool value
- **`player`**: Contains a `property` corresponding to a [player property](#player-properties)
- **`level`**: Contains a `property` corresponding to a [level property](#level-properties)
- Note: There are not yet any boolean properties made available from entities
- **`and`**: Perform and on `lhs` and `rhs`
- **`or`**: Perform or on `lhs` and `rhs`
- **`xor`**: Perform xor on `lhs` and `rhs`
- **`not`**: Perform not on `val`
- **`var` (`variable`)**: Get a variable `name` (StringProperty) from the calling entity
- **`condition` (`from_condition`)**: Return true or false based on whether a [Condition](#conditions) `condition` is met

## Player Properties
- `x` (int)
- `y` (int)
- `height` (int)
- `dreaming` (bool)
- `layer` (int)
- `check_walkable` (bool) (i dont know what this does)

## Level Properties
- `default_x` (int)
- `default_y` (int)
- `tint_r` (int)
- `tint_g` (int)
- `tint_b` (int)
- `special_save_game` (bool)
- `paused` (bool)
- `background_r`: (int)
- `background_g`: (int)
- `background_b`: (int)

## Entity Properties
These are only valid to access from an entity action 
- `x` (int)
- `y` (int)
- `id` (int)
- `draw` (bool)

# Conditions
A condition begins with a `type`
- `int_equals`: Compare IntProperty `lhs` == `rhs`
- `int_greater`: Compare IntProperty `lhs` > `rhs`
- `int_less`: Compare IntProperty `lhs` < `rhs`
- `string_equals`: Compare StringProperty `lhs` == `rhs`
- `effect_equipped`: Check if `effect` (string) is equipped
- `negate`: Negate `condition` (Condition)
- `bool`: Check if BoolProperty `val` is true (or if IntProperty `val` == 1)
- `variable`: Check if variable `name` is true
