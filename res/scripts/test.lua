local frame = 0

function _update(world, this)
    frame = frame + 1
    -- print(world:width())
end

function _load(world, this)
    print("Hello from load! Frame is " .. tostring(frame))
end

local function flip(direction)
    if direction == "up" then
        return "down"
    elseif direction == "down" then
        return "up"
    elseif direction == "left" then
        return "right"
    else 
        return "left"
    end
end

function _use(world, this, direction)
    print("Hello from use! Frame is " .. tostring(frame))
    this:walk(flip(direction))
end

function _walk(world, this, direction)
    print("Hello from walk! Frame is " .. tostring(frame))
end

function _bump(world, this, direction)
    print("Hello from bump! Frame is " .. tostring(frame))
end