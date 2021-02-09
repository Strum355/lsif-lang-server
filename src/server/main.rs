use std::error::Error;

use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::{
    request::GotoDefinition, GotoDefinitionResponse, InitializeParams, Location, Position, Range,
    ServerCapabilities, Url,
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("Server starting...");

    let (connection, io_threads) = Connection::stdio();
    eprintln!("Created connection");

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities::default()).unwrap();
    eprintln!("Server Capabilities: {:?}", server_capabilities);

    let initialize_params = connection.initialize(server_capabilities)?;

    eprintln!("Calling main loop");
    main_loop(&connection, initialize_params)?;
    io_threads.join()?;

    eprintln!("Shutting down server");

    Ok(())
}

fn main_loop(
    connection: &Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    eprintln!("Begin running loop...");

    for msg in &connection.receiver {
        eprintln!("got msg: {:?}", msg);
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                match cast::<GotoDefinition>(req) {
                    Ok((id, _params)) => {
                        eprintln!("Yo, got goto");
                        let result = Some(GotoDefinitionResponse::Scalar(Location {
                            uri: Url::from_file_path("/tmp/file.txt").expect("file"),
                            range: Range {
                                start: Position {
                                    line: 1,
                                    character: 1,
                                },
                                end: Position {
                                    line: 1,
                                    character: 1,
                                },
                            },
                        }));
                        let result = serde_json::to_value(&result).unwrap();
                        let resp = Response {
                            id,
                            result: Some(result),
                            error: None,
                        };
                        connection.sender.send(Message::Response(resp))?;
                        continue;
                    }
                    Err(_) => {}
                }
            }
            Message::Response(_) => {}
            Message::Notification(_) => {}
        }
    }

    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), Request>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
