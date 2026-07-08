local warp_map
local warp_x
local warp_y
local start_frame

local close_timer = 0

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    warp_map = this:meta("warp_map") or "nexus.tmx"
    warp_x = this:meta("warp_x") or 0
    warp_y = this:meta("warp_y") or 0
    start_frame = this:meta("start_frame") or 0
    this.frame = start_frame
    
    -- if the player spawns directly below the door, assume they came from the other side
    if (player.y - this.y) // 16 == 1 and (player.x - this.x) // 16 == 0 then
        print("detected")
        player.frozen = true
        this.frame = start_frame + 2
        close_timer = 64
    end
end

local timer = 0

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    if close_timer > 0 or timer > 0 then
        return
    end
    if direction == Directions.Down then
        player.frozen = true
        world:play("small_fridge_open_001", 1.0, 2.0)
        timer = 32
    end
end

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if timer > 0 then
        timer = timer - 1

        if timer == 24 then
            this.frame = this.frame + 1
        end

        if timer == 16 then
            world:play("space", 1.0, 2.0)
            this.frame = this.frame + 1
        end

        if timer == 0 then
            local trans = Transition.new()
            trans.type = "zoom"
            trans.speed = 2
            trans.scale = 5.0
            world:change_map(
                warp_map,
                trans,
                warp_x,
                warp_y
            )
        end
    end

    if close_timer > 0 then
        close_timer = close_timer - 1

        if close_timer == 16 then
            world:play("small_fridge_open_001", 0.9, 1.0)
        end

        if close_timer == 4 then
            this.frame = this.frame - 1
        end

        if close_timer == 0 then
            this.frame = this.frame - 1
            player.frozen = false
        end
    end
end