//! gRPC server reflection's messages: what a server that offers it is asked,
//! and what it answers. With them, a server's `.proto` files are fetched from
//! the server itself. See `grpc/reflection/v1/reflection.proto`.

use prost::Message;

/// The call, in the version most servers have, and the older one some still
/// have only.
pub const PATHS: [&str; 2] = [
    "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
    "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
];

#[derive(Clone, PartialEq, Message)]
pub struct Request {
    #[prost(string, tag = "1")]
    pub host: String,
    #[prost(oneof = "Ask", tags = "3, 4, 7")]
    pub ask: Option<Ask>,
}

#[derive(Clone, PartialEq, prost::Oneof)]
pub enum Ask {
    #[prost(string, tag = "3")]
    FileByFilename(String),
    #[prost(string, tag = "4")]
    FileContainingSymbol(String),
    #[prost(string, tag = "7")]
    ListServices(String),
}

#[derive(Clone, PartialEq, Message)]
pub struct Response {
    #[prost(oneof = "Answer", tags = "4, 6, 7")]
    pub answer: Option<Answer>,
}

#[derive(Clone, PartialEq, prost::Oneof)]
pub enum Answer {
    #[prost(message, tag = "4")]
    Files(Files),
    #[prost(message, tag = "6")]
    Services(Services),
    #[prost(message, tag = "7")]
    Error(Error),
}

#[derive(Clone, PartialEq, Message)]
pub struct Files {
    /// Each a `FileDescriptorProto`.
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub file_descriptor_proto: Vec<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Services {
    #[prost(message, repeated, tag = "1")]
    pub service: Vec<Service>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Service {
    #[prost(string, tag = "1")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct Error {
    #[prost(int32, tag = "1")]
    pub error_code: i32,
    #[prost(string, tag = "2")]
    pub error_message: String,
}

/// Messages, each in its gRPC frame.
pub fn frames<M: Message>(messages: &[M]) -> Vec<u8> {
    let mut out = Vec::new();
    for message in messages {
        let bytes = message.encode_to_vec();
        out.push(0);
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(&bytes);
    }
    out
}

/// The messages in gRPC frames, or `None` if one cannot be read.
pub fn unframe<M: Message + Default>(mut data: &[u8]) -> Option<Vec<M>> {
    let mut out = Vec::new();
    while !data.is_empty() {
        let (flags, payload, after) = crate::protobuf::frame(data)?;
        if flags & 0x80 == 0 {
            let payload = if flags & 1 != 0 {
                crate::protobuf::unpack(payload)?
            } else {
                payload.to_vec()
            };
            out.push(M::decode(&payload[..]).ok()?);
        }
        data = after;
    }
    Some(out)
}

/// What a server offering reflection answers: the services its files have,
/// and the files that hold a symbol or have a name, with all they import.
/// For a test server, and to see that what is asked is understood.
pub fn answer(files: &prost_types::FileDescriptorSet, request: &Request) -> Response {
    use std::collections::BTreeSet;
    let file = |name: &str| files.file.iter().find(|f| f.name() == name);
    // A file and those it imports, the file first.
    let with_imports = |first: &prost_types::FileDescriptorProto| {
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        let mut todo = vec![first.clone()];
        while let Some(f) = todo.pop() {
            if seen.insert(f.name().to_owned()) {
                todo.extend(f.dependency.iter().filter_map(|d| file(d).cloned()));
                order.push(f.encode_to_vec());
            }
        }
        Answer::Files(Files {
            file_descriptor_proto: order,
        })
    };
    let not_found = |what: &str| {
        Answer::Error(Error {
            error_code: 5,
            error_message: format!("{what} not found"),
        })
    };
    let answer = match &request.ask {
        Some(Ask::ListServices(_)) => Answer::Services(Services {
            service: files
                .file
                .iter()
                .flat_map(|f| {
                    let package = f.package().to_owned();
                    f.service.iter().map(move |s| Service {
                        name: if package.is_empty() {
                            s.name().to_owned()
                        } else {
                            format!("{package}.{}", s.name())
                        },
                    })
                })
                .collect(),
        }),
        Some(Ask::FileByFilename(name)) => match file(name) {
            Some(f) => with_imports(f),
            None => not_found(name),
        },
        Some(Ask::FileContainingSymbol(symbol)) => {
            let holds = |f: &&prost_types::FileDescriptorProto| {
                let package = f.package();
                let full = |name: &str| {
                    if package.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{package}.{name}")
                    }
                };
                f.service.iter().any(|s| full(s.name()) == *symbol)
                    || f.message_type.iter().any(|m| full(m.name()) == *symbol)
                    || f.enum_type.iter().any(|e| full(e.name()) == *symbol)
            };
            match files.file.iter().find(holds) {
                Some(f) => with_imports(f),
                None => not_found(symbol),
            }
        }
        None => not_found("nothing asked"),
    };
    Response {
        answer: Some(answer),
    }
}
