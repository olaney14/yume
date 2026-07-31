local nerve = require("res.scripts.nerve_paths.nerve_paths")

local delay = 0

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    nerve.enable_movement = true
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    if delay == 0 then
        delay = 60
        world:play("shock", 1.5, 0.5)
        nerve.enable_movement = not nerve.enable_movement
    end
end

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if delay > 0 then
        delay = delay - 1
    end
end