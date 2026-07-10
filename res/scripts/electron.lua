function quantize(num, to)
    return math.floor(num * to)
end

local facing = Directions.Down

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player) 
    math.randomseed(quantize(world:session_random(), 255))
    this.speed = 2
end

-- these are the only ones actually used in the map so far
local directions_table = {
    [0] = {Directions.Down, Directions.Right},
    [1] = {Directions.Left, Directions.Right},
    [2] = {Directions.Down, Directions.Left},
    [8] = {Directions.Up, Directions.Down},
    [9] = {Directions.Up, Directions.Left},
    [16] = {Directions.Up, Directions.Right},
    [33] = {Directions.Up, Directions.Left, Directions.Right},
    [56] = {Directions.Up, Directions.Down, Directions.Left, Directions.Right}
}

local rotate_cw = {
    [Directions.Up] = Directions.Right,
    [Directions.Right] = Directions.Down,
    [Directions.Down] = Directions.Left,
    [Directions.Left] = Directions.Up
}

function contains(table, value)
    for _, v in pairs(table) do
        if v == value then
            return true
        end
    end

    return false
end

---@param direction Direction
function pass_between_tiles(a, b, direction)
    if not directions_table[a:id()] or not directions_table[b:id()] then
        return false
    end

    return contains(directions_table[a:id()], direction) and contains(directions_table[b:id()], direction:flipped())
end

local all_directions = {
    Directions.Up,
    Directions.Down,
    Directions.Left,
    Directions.Right
}

---@param world World
function check_side_valid(world, this, side)
    local from = world:get_tiles(this.x // 16, this.y // 16, this.layer)
    local to_x, to_y = world:wrap_tile((this.x // 16) + side:x(), (this.y // 16) + side:y()) 
    local to = world:get_tiles(to_x, to_y, this.layer)

    for _, v in pairs(from) do
        for _, w in pairs(to) do
            if v:id() == -1 or w:id() == -1 then
                goto continue -- ???? lua
            end
            if pass_between_tiles(v, w, side) then
                return true
            end
            ::continue::
        end
    end

    return false
end

-- https://stackoverflow.com/questions/35572435/how-do-you-do-the-fisher-yates-shuffle-in-lua
local function ShuffleInPlace(t)
    for i = #t, 2, -1 do
        local j = math.random(i)
        t[i], t[j] = t[j], t[i]
    end
end

---@param world World
---@param this Entity
function try_move(world, this)
    local valid_moves = {}
    local turn_back = nil

    for _, dir in pairs(all_directions) do
        -- avoid where we came from except for dead ends
        if dir:flipped() == facing then
            turn_back = dir
        else
            if check_side_valid(world, this, dir) then
                table.insert(valid_moves, dir)
            end
        end
    end

    if #valid_moves > 0 then
        local move = valid_moves[math.random(1, #valid_moves)]
        facing = move
        this:walk_noclip(move)
        return
    end

    if turn_back then
        facing = turn_back
        this:walk_noclip(turn_back)
    end
end

local anim_timer = 4

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if not this:moving() then
        -- for _ = 0, 3, 1 do
        --     if not try_move(world, this) then
        --         facing = rotate_cw[facing]
        --     else
        --         break
        --     end
        -- end
        try_move(world, this)
    end

    anim_timer = anim_timer - 1

    if anim_timer == 0 then
        anim_timer = 4
        if this.frame == 0 then
            this.frame = 1
        else
            this.frame = 0
        end
    end
end