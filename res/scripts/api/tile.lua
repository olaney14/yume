-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return
---@diagnostic disable: missing-fields

---@class Tile
local Tile = {}

--- Return the id of this tile within its tileset
---@param self Tile
---@return integer
function Tile:id() end

--- Return the id of this tile's tileset
--- @param self Tile
--- @return integer
function Tile:tileset() end

--- Return true if this tile blocks movement
--- @param self Tile
--- @return boolean
function Tile:blocking() end

return Tile