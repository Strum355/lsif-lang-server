local json_decode = vim.fn.json_decode
local json_encode = vim.fn.json_encode
local schedule = vim.schedule
local protocol = vim.lsp.protocol
local validate = vim.validate

local format_rpc_error = vim.lsp.rpc.format_rpc_error
local rpc_response_error = vim.lsp.rpc.rpc_response_error

local header_start_pattern = ("content"):gsub("%w", function(c) return "["..c..c:upper().."]" end)

local Job = require('plenary.job')
local log = require('plenary.log')

local convert_NIL = require('vim_utils').convert_NIL

local lib = {}

local client_errors = vim.tbl_add_reverse_lookup {
  INVALID_SERVER_MESSAGE       = 1;
  INVALID_SERVER_JSON          = 2;
  NO_RESULT_CALLBACK_FOUND     = 3;
  READ_ERROR                   = 4;
  NOTIFICATION_HANDLER_ERROR   = 5;
  SERVER_REQUEST_HANDLER_ERROR = 6;
  SERVER_RESULT_CALLBACK_ERROR = 7;
}

local on_error = function(code, err, ...)
  print("ERROR:", client_errors[code], err, ...)
end

local function pcall_handler(errkind, status, head, ...)
  if not status then
    on_error(errkind, head, ...)
    return status, head
  end

  return status, head, ...
end

--@private
local function try_call(errkind, fn, ...)
  return pcall_handler(errkind, pcall(fn, ...))
end

local function parse_headers(header)
  if type(header) ~= 'string' then
    return nil
  end
  local headers = {}
  for line in vim.gsplit(header, '\r\n', true) do
    if line == '' then
      break
    end
    local key, value = line:match("^%s*(%S+)%s*:%s*(.+)%s*$")
    if key then
      key = key:lower():gsub('%-', '_')
      headers[key] = value
    else
      local _ = log.error() and log.error("invalid header line %q", line)
      error(string.format("invalid header line %q", line))
    end
  end
  headers.content_length = tonumber(headers.content_length)
      or error(string.format("Content-Length not found in headers. %q", header))
  return headers
end

function lib.format_message_with_content_length(msg)
  local encoded_message = json_encode(msg)

  return table.concat {
    'Content-Length: '; tostring(#encoded_message); '\r\n\r\n';
    encoded_message;
  }
end

function lib.handle_body(client, body)
  local dispatchers = client.dispatchers
  local message_callbacks = client.message_callbacks

  local decoded, err = json_decode(body)
  if not decoded then
    -- on_error(client_errors.INVALID_SERVER_JSON, err)
    return
  end
  local _ = log.debug() and log.debug("decoded", decoded)

  if type(decoded.method) == 'string' and decoded.id then
    -- Server Request
    decoded.params = convert_NIL(decoded.params)
    -- Schedule here so that the users functions don't trigger an error and
    -- we can still use the result.
    schedule(function()
      local status, result
      status, result, err = try_call(
        client_errors.SERVER_REQUEST_HANDLER_ERROR,
          dispatchers.server_request,
          decoded.method,
          decoded.params)

      local _ = log.debug() and log.debug("server_request: callback result", { status = status, result = result, err = err })
      if status then
        if not (result or err) then
          -- TODO this can be a problem if `null` is sent for result. needs vim.NIL
          error(string.format("method %q: either a result or an error must be sent to the server in response", decoded.method))
        end
        if err then
          assert(type(err) == 'table', "err must be a table. Use rpc_response_error to help format errors.")
          local code_name = assert(protocol.ErrorCodes[err.code], "Errors must use protocol.ErrorCodes. Use rpc_response_error to help format errors.")
          err.message = err.message or code_name
        end
      else
        -- On an exception, result will contain the error message.
        err = rpc_response_error(protocol.ErrorCodes.InternalError, result)
        result = nil
      end

      -- send_response(decoded.id, err, result)
      print("TODO TODO TODO")
    end)
  -- This works because we are expecting vim.NIL here
  elseif decoded.id and (decoded.result or decoded.error) then
    -- Server Result
    decoded.error = convert_NIL(decoded.error)
    decoded.result = convert_NIL(decoded.result)

    -- Do not surface RequestCancelled or ContentModified to users, it is RPC-internal.
    if decoded.error then
      if decoded.error.code == protocol.ErrorCodes.RequestCancelled then
        local _ = log.debug() and log.debug("Received cancellation ack", decoded)
      elseif decoded.error.code == protocol.ErrorCodes.ContentModified then
        local _ = log.debug() and log.debug("Received content modified ack", decoded)
      end
      local result_id = tonumber(decoded.id)
      -- Clear any callback since this is cancelled now.
      -- This is safe to do assuming that these conditions hold:
      -- - The server will not send a result callback after this cancellation.
      -- - If the server sent this cancellation ACK after sending the result, the user of this RPC
      -- client will ignore the result themselves.
      if result_id then
        message_callbacks[result_id] = nil
      end
      return
    end

    -- We sent a number, so we expect a number.
    local result_id = tonumber(decoded.id)
    local callback = message_callbacks[result_id]
    if callback then
      message_callbacks[result_id] = nil
      validate {
        callback = { callback, 'f' };
      }
      if decoded.error then
        decoded.error = setmetatable(decoded.error, {
          __tostring = format_rpc_error;
        })
      end
      try_call(client_errors.SERVER_RESULT_CALLBACK_ERROR,
          callback, decoded.error, decoded.result)
    else
      log.debug("No callback for method result_id: " .. result_id)

      -- Original code:
      --
      -- on_error(client_errors.NO_RESULT_CALLBACK_FOUND, decoded)
      -- local _ = log.error() and log.error("No callback found for server response id "..result_id)
    end
  elseif type(decoded.method) == 'string' then
    -- Notification
    decoded.params = convert_NIL(decoded.params)
    try_call(client_errors.NOTIFICATION_HANDLER_ERROR,
        dispatchers.notification, decoded.method, decoded.params)
  else
    -- Invalid server message
    on_error(client_errors.INVALID_SERVER_MESSAGE, decoded)
  end
end

local function request_parser_loop()
  local buffer = '' -- only for header part
  while true do
    -- A message can only be complete if it has a double CRLF and also the full
    -- payload, so first let's check for the CRLFs
    local start, finish = buffer:find('\r\n\r\n', 1, true)
    -- Start parsing the headers
    if start then
      -- This is a workaround for servers sending initial garbage before
      -- sending headers, such as if a bash script sends stdout. It assumes
      -- that we know all of the headers ahead of time. At this moment, the
      -- only valid headers start with "Content-*", so that's the thing we will
      -- be searching for.
      -- TODO(ashkan) I'd like to remove this, but it seems permanent :(
      local buffer_start = buffer:find(header_start_pattern)
      local headers = parse_headers(buffer:sub(buffer_start, start-1))
      local content_length = headers.content_length
      -- Use table instead of just string to buffer the message. It prevents
      -- a ton of strings allocating.
      -- ref. http://www.lua.org/pil/11.6.html
      local body_chunks = {buffer:sub(finish+1)}
      local body_length = #body_chunks[1]
      -- Keep waiting for data until we have enough.
      while body_length < content_length do
        local chunk = coroutine.yield()
            or error("Expected more data for the body. The server may have died.") -- TODO hmm.
        table.insert(body_chunks, chunk)
        body_length = body_length + #chunk
      end
      local last_chunk = body_chunks[#body_chunks]

      body_chunks[#body_chunks] = last_chunk:sub(1, content_length - body_length - 1)
      local rest = ''
      if body_length > content_length then
        rest = last_chunk:sub(content_length - body_length)
      end
      local body = table.concat(body_chunks)
      -- Yield our data.
      buffer = rest..(coroutine.yield(headers, body)
          or error("Expected more data for the body. The server may have died.")) -- TODO hmm.
    else
      -- Get more data since we don't have enough.
      buffer = buffer..(coroutine.yield()
          or error("Expected more data for the header. The server may have died.")) -- TODO hmm.
    end
  end
end

function lib.start_server()
  local request_parser = coroutine.wrap(request_parser_loop)
  request_parser()

  local message_callbacks = {}

  local j = Job:new {
    command = './target/debug/server',
    on_stderr = vim.schedule_wrap(function(_, msg)
      -- print("STDERR", msg)
    end),

    on_exit = vim.schedule_wrap(function(_, code, signal)
      print("EXIT:", code, signal)
    end),

    env = {
      RUST_BACKTRACE = 1,
    }
  }


  j:start {}

  -- Just attach directly.
  j.stdout:read_start(vim.schedule_wrap(function(err, chunk)
    if err then
      -- TODO better handling. Can these be intermittent errors?
      on_error(client_errors.READ_ERROR, err)
      return
    end

    -- This should signal that we are done reading from the client.
    if not chunk then return end
    -- Flush anything in the parser by looping until we don't get a result
    -- anymore.
    while true do
      local headers, body = request_parser(chunk)
      -- If we successfully parsed, then handle the response.
      if headers then
        -- TODO: Handle different client things.
        lib.handle_body({
          dispatchers = {},
          message_callbacks = message_callbacks,
        }, body)
        -- Set chunk to empty so that we can call request_parser to get
        -- anything existing in the parser to flush.
        chunk = ''
      else
        break
      end
    end
  end))


  j._request_id = 0

  j.request = function(_, arg, callback)
    j._request_id = j._request_id + 1

    arg.jsonrpc = "2.0"
    arg.id = j._request_id

    message_callbacks[j._request_id] = callback

    log.trace("lsp_lib_request:", arg)
    j:send(lib.format_message_with_content_length(arg))
  end

  j.request_sync = function(_, arg, timeout_ms)
    timeout_ms = timeout_ms or 1000

    local result, done = nil, false
    local function _sync_handler(err, res)
      result = { error = err, result = res }
      done = true
    end

    j:request(arg, _sync_handler)

    vim.wait(timeout_ms, function() return done end)

    return result
  end

  j.notify = function(_, arg)
    arg.jsonrpc = "2.0"

    log.trace("lsp_lib_notify:", arg)
    j:send(lib.format_message_with_content_length(arg))
  end

  j.initialize = function(_, capabilities)
    capabilities = vim.tbl_deep_extend("force", {
      textDocument = {
        hover = {
          dynamicRegistration = false,
          contentFormat = {
            'plaintext',
          }
        }
      }
    }, capabilities or {})

    j:request {
      method = "initialize",
      params = {
        capabilities = capabilities,
      }
    }

    j:notify {
      method = "initialized",
      params = {}
    }
  end

  j.finish = function()
    j:request {
      method = "shutdown"
    }

    j:notify {
      method = "exit"
    }
  end

  return j
end


return lib
