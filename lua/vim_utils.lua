
local M = {}

local NIL = vim.NIL

--@private
local recursive_convert_NIL
recursive_convert_NIL = function(v, tbl_processed)
  if v == NIL then
    return nil
  elseif not tbl_processed[v] and type(v) == 'table' then
    tbl_processed[v] = true
    return vim.tbl_map(function(x)
      return recursive_convert_NIL(x, tbl_processed)
    end, v)
  end

  return v
end

--@private
--- Returns its argument, but converts `vim.NIL` to Lua `nil`.
--@param v (any) Argument
--@returns (any)
function M.convert_NIL(v)
  return recursive_convert_NIL(v, {})
end

return M
