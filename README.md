# yume
yume nikki fangame with Rust + SDL

# guide for tiled properties
## map
### clampCamera (bool)
### defaultPos (string)
default pos for player, used when debug warping and in the `default` warp coordinate. formatted as `x,y`
### edges (string)
JSON string that defines an action for each side
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
### looping (bool)
### music (string)
path from res to the map's music
### music_speed (float)
### music_volume (float)
### tint (string)
the map's tint color, formatted as `r,g,b,a`
## layer
### height (int)
### draw (bool)
### collide (bool)
### name (string)
layer's name for access by `World::get_mut_layer_by_name`
## entities
### height (int)
### solid (bool)
### draw (bool)
### walk_behind (bool)
if this is true, when the player is above the object it will be draw above the player
### collider (string)
JSON object with `x`, `y`, `w`, `h` properties
```
{
	"x": 0,
	"y": 16,
	"w": 16,
	"h": 16
}
```
### ai (string)
JSON string, see AI section
### animation (string)
JSON string, see animation section
### actions (string)
JSON string, an array with objects with a `trigger` and `action` (see action section)
```
	"actions": [{
		"trigger": {
			"type": "use"
		},
		"action": {
			"type": "delayed",
			"action": {
				"type": "warp",
				"map": "$warp_map",
				"pos": {
					"x": "$warp_x",
					"y": "$warp_y"
				},
				"transition": "fade"
			},
			"delay": 16
		}
	},
	{
		"trigger": {
			"type": "use"
		},
		"action": {
			"type": "freeze"
		}
	}]
```
### file (string)
path to a JSON file to load as properties onto this entity
a string beginning with a $ will be replaced with the according tiled property
ex: `"x": "$warp_x"` would be replaced by the property `warp_x` in the entity's tiled properties
## ai
An ai entry is a json object in a file or string property
```
"ai": {
  "type": "chaser",
  "speed": "$chaser_speed", 
  "detection_radius": 100
},
```
All ai entries start with a `type`
### AI Type: `wander`
The entity wanders around randomly
#### frequency (i32)
every frame, if rand from 0 to frequency == 0 and past delay time, move, defaults to 100
#### delay (i32)
min time for entity to move, defaults to 25
### AI Type: `chaser`
The entity chases the player using an A* pathfinder
When the entity bumps into the player, a player bump interaction is simulated
#### speed (u32)
player's default speed is 1, with running shoes is 2, defaults to 1
#### path_max (u32)
max amount of steps the chaser's pathfinding can search, defaults to 10000
#### detection_radius (u32)
the maximum taxicab distance the chaser can chase after the player from, defaults to 16
### AI Type: `push`
the entity is pushed on interaction
#### speed (u32)
defaults to 2
### AI Type: `animate_on_interact`
#### frames (u32)
number of frames to advance, defaults to 1
#### use (bool)
#### bump (bool)
#### walk (bool)
these properties define if the animation can be triggered by a use, bump, or walk interaction, all default to false
#### side (string) (optional)
if present, only a certain side will trigger
## animators
an animator is similar to an ai definition
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
