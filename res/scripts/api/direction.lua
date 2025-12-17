-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return
---@diagnostic disable: missing-fields

---@class Direction
local Direction = {}

---@param self Direction
---@return integer
function Direction:x() end

---@param self Direction
---@return integer
function Direction:y() end

---@param self Direction
---@return string
function Direction:tostring() end

---@param self Direction
---@return Direction
function Direction:flipped() end

---@class Directions
---@field Up Direction
---@field Down Direction
---@field Left Direction
---@field Right Direction

---@type Directions
Directions = {}

return Direction