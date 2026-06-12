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

---@param self Player
---@return boolean
function Player:moving() end

---@param self Player
---@return integer
function Player:speed() end

---@param self Player
---@return integer
function Player:sub_speed() end

---@param self Player
---@return boolean
function Player:dreaming() end

---@param self Player
---@return number
function Player:random() end

---@param self Player
---@return integer
function Player:frame() end

return Player