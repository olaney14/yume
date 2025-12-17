local chase = true

function abs_diff(x, y)
    return math.abs(x - y)
end

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    this.speed = this:meta("speed") or 1
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    world:play("click", 1.0, 1.0)
    chase = not chase
end

---@param world World
---@param this Entity
---@param player Player
function move(world, this, player)
    -- 0: this
    -- 1: player
    local standing_y = player.y + 16
    local diff_x = world:looped_distance_x(this.x, player.x)
    local diff_y = world:looped_distance_y(this.y, standing_y)

    if not this:moving() then
        local direction

        -- Move along furthest axis
        if diff_x > diff_y then
            if diff_x == 0 then return end
            if player.x - this.x > 0 then direction = Directions.Right
            else direction = Directions.Left end
            if world:looping_x() and diff_x ~= abs_diff(this.x, player.x) then direction = direction:flipped() end
        else
            if diff_y == 0 then return end
            if standing_y - this.y > 0 then direction = Directions.Down
            else direction = Directions.Up end
            if world:looping_y() and diff_y ~= abs_diff(this.y, standing_y) then direction = direction:flipped() end
        end

        local check_x = this.x + direction:x() * 16
        local check_y = this.y + direction:y() * 16
        local tile_x, tile_y = world:wrap_tile(check_x / 16, check_y / 16)

        -- Move along walls
        -- print("try at " .. tostring(tile_x) .. ", " .. tostring(tile_y))
        if world:collide_tile(tile_x, tile_y, this.layer) then
            -- print("collided at " .. tostring(tile_x) .. ", " .. tostring(tile_y))
            if direction == Directions.Left or direction == Directions.Right then
                if diff_y == 0 then return end
                if standing_y - this.y > 0 then direction = Directions.Down
                else direction = Directions.Up end
                if world:looping_y() and diff_y ~= abs_diff(this.y, standing_y) then direction = direction:flipped() end
            else
                if diff_x == 0 then return end
                if player.x - this.x > 0 then direction = Directions.Right
                else direction = Directions.Left end
                if world:looping_x() and diff_x ~= abs_diff(this.x, player.x) then direction = direction:flipped() end
            end
        end

        this:walk(direction)
    end
end

-- adapted from WalkTowardsPathfinder::poll()

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if chase then
        move(world, this, player)
    end
end