---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    this.solid = false
    this.walk_over = true
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _walk(world, this, player, direction)
    local path = this:meta("world")
    local x = this:meta("x")
    local y = this:meta("y")

    local transition = Transition.new()
    
    world:change_map(path, transition, x, y)
end