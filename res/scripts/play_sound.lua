local last_used = 0

local sound
local speed
local volume
local delay

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    sound = this:meta("sound")
    speed = this:meta("speed") or 1.0
    volume = this:meta("volume") or 1.0
    delay = this:meta("delay") or 1.0
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    local diff = os.clock() - last_used

    if diff > delay then
        world:play(sound, speed, volume)
        last_used = os.clock()
    end
end