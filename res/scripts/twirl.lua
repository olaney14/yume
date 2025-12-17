local ox = 0
local oy = 0
local time = 0

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    this.x = ox + math.cos(time / 10) * 32
    this.y = oy + math.sin(time / 10) * 32
    
    time = time + 1
end

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    ox = this.x;
    oy = this.y;
end