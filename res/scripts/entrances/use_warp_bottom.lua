local warp_map
local warp_x
local warp_y

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    warp_map = this:meta("warp_map") or "nexus.tmx"
    warp_x = this:meta("warp_x") or 0
    warp_y = this:meta("warp_y") or 0
    this.walk_over = true
end

local warp_timer = 0

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)    
    if direction == Directions.Down then
        -- to mirror data warp: freeze, wait 8 frames, fade warp with speed 2 hold 4
        player.frozen = true
        warp_timer = 8
    end
end

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if warp_timer > 0 then
        warp_timer = warp_timer - 1

        if warp_timer == 0 then
            local transition = Transition.new()
            transition.speed = 2
            transition.hold = 4
            world:change_map(warp_map, transition, warp_x, warp_y)
        end
    end
end