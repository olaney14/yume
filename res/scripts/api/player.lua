-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return

---@class Player
---@field x integer
---@field y integer
---@field frozen boolean
---@field money integer
---@field layer integer
---@field facing Direction
local Player = {}

--- Return true if the player is moving
---@param self Player
---@return boolean
function Player:moving() end

--- Return the player's speed
---@param self Player
---@return integer
function Player:speed() end

--- Return the sub_speed: delay in frames between movement
--- essentially dividing the speed
---@param self Player
---@return integer
function Player:sub_speed() end

--- Return true if the player is in the dream world
---@param self Player
---@return boolean
function Player:dreaming() end

--- Return a random number on [0.0,1.0) unique to the save file
---@param self Player
---@return number
function Player:random() end

--- Return the player's current animation frame
---@param self Player
---@return integer
function Player:frame() end

return Player