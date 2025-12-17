-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return

---Called every frame for entity scripts
---@overload fun(world: World, this: Entity, player: Player)
function _update(world, this, player) end

---@overload fun(world: World, this: Entity, player: Player, direction: Direction)
function _use(world, this, player, direction) end