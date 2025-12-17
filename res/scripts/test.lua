local frame = 0

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    frame = frame + 1
    -- print(world:width())
end

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    print("Hello from load! Frame is " .. tostring(frame))
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    print("Hello from use! Frame is " .. tostring(frame))
    -- print("World: " .. tostring(world))
    -- print("This: " .. tostring(this))
    -- print("Direction: " .. tostring(direction))

    this:walk(direction:flipped())
    -- this:walk()
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _walk(world, this, player, direction)
    print("Hello from walk! Frame is " .. tostring(frame))
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _bump(world, this, player, direction)
    print("Hello from bump! Frame is " .. tostring(frame))
end