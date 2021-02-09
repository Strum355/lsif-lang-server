vim.cmd [[set rtp+=.]]

local lib = require('lib_lsp')

local eq = assert.are.same

local j = nil

describe('lisf-protocol-rs', function()
  before_each(function()
    j = lib.start_server()

    j:initialize()
  end)

  after_each(function()
    j:finish()
  end)

  it('can connect', function()
    assert(not j.is_shutdown)
  end)

  it('can get a definition', function()
    assert(not vim.tbl_isempty(j:request_sync {
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
    }))
  end)
end)
