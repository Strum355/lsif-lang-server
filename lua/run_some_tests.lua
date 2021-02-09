-- local uv = vim.loop

-- local stdin = uv.new_pipe(false)
-- local stdout = uv.new_pipe(false)
-- local stderr = uv.new_pipe(false)

-- local job = vim.loop.spawn(
--   'cargo', {
--     args = {'run', 'server'},

--     stdin = stdin,
--     stdout = stdout,
--     stderr = stderr,
--   }, function(code, signal)
--     print("Exit:", code, signal)
--   end
-- )

-- P(job.stdout)


local Job = require('plenary.job')
local lib = R('lib_lsp')

Job:new { command = 'cargo', args = { 'build' } }:sync()

local j = lib.start_server()

j:initialize {}

j:request_sync {
  method = "textDocument/definition",
  params = {
    textDocument = {
      uri = "file://tmp"
    },
    position = {
      line = 1,
      character = 1,
    },
  }
}

j:finish()
