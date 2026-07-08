function quantize(num, to)
    return math.floor(num * to)
end

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player) 
    math.randomseed(quantize(world:session_random(), 255))
    this.speed = 1
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    world:give_effect("rabbit")
end

local timer = 50
local move_timer = 0
local direction = Directions.Down

function random_direction()
    local d = math.random(4)
    if d == 1 then
        return Directions.Up
    elseif d == 2 then
        return Directions.Down
    elseif d == 3 then
        return Directions.Left
    end
    return Directions.Right
end

local step_timer = 18

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if move_timer > 0 then
        if not this:moving() then
            this:walk(direction)
        end
        move_timer = move_timer - 1
    end

    if timer > 0 then
        timer = timer - 1
    else
        if math.random(50) == 1 then
            -- this:walk(random_direction())
            timer = 150
            step_timer = 1
            direction = random_direction()
            move_timer = math.random(1, 240)
        end
    end

    if step_timer > 0 and this:moving() then
        step_timer = step_timer - 1
        if step_timer == 0 then
            local dist = math.sqrt((this.x - player.x) ^ 2 + (this.y - player.y) ^ 2) / 16
            local vol = math.max(math.min(1, -dist / 10 + 1.1), 0)
            world:play("step_soft", 1.0, vol)
            step_timer = 18
        end
    end
end