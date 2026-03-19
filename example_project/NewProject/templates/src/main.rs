use std::io::Read;

mod templates;

use toki_templates::{
    TemplateProviderRequest, TemplateProviderResponse, TEMPLATE_PROTOCOL_VERSION,
};

fn main() {
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();
    let request: TemplateProviderRequest = serde_json::from_str(&stdin).unwrap();

    let response = match request {
        TemplateProviderRequest::List { .. } => TemplateProviderResponse::List {
            protocol_version: TEMPLATE_PROTOCOL_VERSION,
            templates: templates::list_templates(),
        },
        TemplateProviderRequest::Describe { template_id, .. } => match templates::describe_template(&template_id) {
            Ok(descriptor) => TemplateProviderResponse::Describe {
                protocol_version: TEMPLATE_PROTOCOL_VERSION,
                descriptor,
            },
            Err(error) => TemplateProviderResponse::Error {
                protocol_version: TEMPLATE_PROTOCOL_VERSION,
                error,
            },
        },
        TemplateProviderRequest::Instantiate { template_id, parameters, .. } => {
            match templates::instantiate_template(&template_id, parameters) {
                Ok(instantiation) => TemplateProviderResponse::Instantiate {
                    protocol_version: TEMPLATE_PROTOCOL_VERSION,
                    descriptor: instantiation.descriptor,
                    plan: instantiation.plan,
                },
                Err(error) => TemplateProviderResponse::Error {
                    protocol_version: TEMPLATE_PROTOCOL_VERSION,
                    error,
                },
            }
        }
    };

    print!("{}", serde_json::to_string(&response).unwrap());
}
