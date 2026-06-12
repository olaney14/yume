---@param world World
---@param this Entity
---@param player Player
function _update(world, this, player)
    this.speed = player:speed()
    if player:moving() and not this:moving() then
        this:walk_noclip(player.facing)
    end

    this.frame = player:frame()
end

---@param world World
---@param this Entity
---@param player Player
function _load(world, this, player)
    -- this script also serves to setup the world for the reflection
    player.layer = 1
    this.x = player.x
    this.y = player.y + 32
end