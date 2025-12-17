-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return
---@diagnostic disable: missing-fields

---@class Transition
---@field kind string
---@field speed integer
---@field delay integer
---@field fade_music boolean
---@field hold integer
---@field reset_music boolean

---@class TransitionClass
---@field new fun(): Transition

---@type TransitionClass
Transition = {}