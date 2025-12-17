local switch = false

---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    if player:moving() and not this:moving() then
        if switch then
            this:walk(player.facing:flipped())
        else
            this:walk(player.facing)
        end
    end
end

---@param world World
---@param this Entity
---@param player Player
---@param direction Direction
function _use(world, this, player, direction)
    switch = not switch
end