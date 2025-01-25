# yume
Yume Nikki fangame with Rust + SDL2

# Table of Contents
- [Properties](#properties)
- [Actions](#actions)
- [Animation](#animation)
- [AI](#ai)
- [Particles](#particles)

# Properties
- [Map](#map)
- [Layer](#layer)
- [Entity](#entity)
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

## AI
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
starts with a `type` 
also starts with a `repeat`
### repeat, defaults to loop
#### cycle
animation cycles back and forth, like 0, 1, 2, 1, 0, 1, 2, ...
the player does this
#### loop
animation just loops, like 0, 1, 2, 0, 1, 2, 0, 1, ...
types:
### still
just stays still (why did i add this????)
#### frame (u32) its the frame it sits still on
### sequence
moves through a sequence of frames
#### start (u32)
#### length (u32)
#### speed (u32)
#### idle (u32) (optional) (useless???)
### directional
#### up (u32)
#### down (u32)
#### left (u32)
#### right (u32)
these define the rows of the animation
#### frames (u32)
how many frames per row
#### speed (u32)
## actions
all begin with a `type`
### `warp`
warps the player
#### map (string) (optional)
path starts from res/maps
#### transition (string)
transition type
#### transition_speed (number) (optional)
#### transition_music (bool) (optional)
#### pos (string)
warp coord things, goes like `"pos": { "x": 5, "y": 5 }`
x and y can be numbers, or several keywords. `match` keeps the player's position for one component, 
`default` puts the player at the map's default position, `sub` + number (ex: `sub32`) takes a position
component and subtracts number, `add` works the same
### `print`
for debugging 
#### message (string)
### `delayed`
delays another action for some amount of frames
#### delay (u32)
#### after (string)
action json
### `freeze`
freezes player
#### time (u32) (optional)
if not present, just toggle freeze on
## tilesets
### blocking (bool)
practical examples:
lamp post
```
{
	"ai": {
		"type": "animate_on_interact",
		"frames": 3,
		"use": true
	},
	"animation": {
		"type": "sequence",
		"start": 0,
		"length": 4,
		"speed": 8,
		"manual": true
	},
	"actions": [{
		"trigger": {
			"type": "use"
		},
		"action": {
			"type": "delayed",
			"action": {
				"type": "warp",
				"map": "$door_warp_map",
				"pos": {
					"x": "$door_warp_x",
					"y": "$door_warp_y"
				},
				"transition": "fade"
			},
			"delay": 64
		}
	},
	{
		"trigger": {
			"type": "use"
		},
		"action": {
			"type": "freeze"
		}
	}],
	"height": "0",
	"solid": true,
	"walk_behind": false
}
```

chaser
```
{
	"ai": {
		"type": "chaser",
		"speed": "$chaser_speed", 
		"detection_radius": 100
	},
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
	"height": "0",
	"solid": true,
	"walk_behind": false
}
```

tall chaser
```
{
	"ai": {
		"type": "chaser",
		"speed": 2,
		"detection_radius": 100
	},
	"animation": {
		"type": "directional",
		"up": 0,
		"down": 1,
		"left": 3,
		"right": 2,
		"frames": 3,
		"speed": 4,
		"repeat": "cycle",
		"on_move": true
	},
	"collider": {
		"x": 0,
		"y": 16,
		"w": 16,
		"h": 16
	},
	"height": "0",
	"solid": true,
	"walk_behind": true
}
```
