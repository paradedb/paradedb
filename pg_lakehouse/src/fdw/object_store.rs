use object_store::aws::{AmazonS3, AmazonS3Builder};
use thiserror::Error;

use super::options::*;

impl TryFrom<ServerOptions> for AmazonS3 {
    type Error = ObjectStoreError;

    fn try_from(options: ServerOptions) -> Result<Self, Self::Error> {
        let server_options = options.server_options();
        let user_mapping_options = options.user_mapping_options();

        let url = require_option(AmazonServerOption::Url.as_str(), server_options)?;
        let region = require_option(AmazonServerOption::Region.as_str(), server_options)?;

        let mut builder = AmazonS3Builder::new().with_url(url).with_region(region);

        if let Some(access_key_id) =
            user_mapping_options.get(AmazonUserMappingOption::AccessKeyId.as_str())
        {
            builder = builder.clone().with_access_key_id(access_key_id.as_str());
        }

        if let Some(secret_access_key) =
            user_mapping_options.get(AmazonUserMappingOption::SecretAccessKey.as_str())
        {
            builder = builder.with_secret_access_key(secret_access_key.as_str());
        }

        if let Some(session_token) =
            user_mapping_options.get(AmazonUserMappingOption::SessionToken.as_str())
        {
            builder = builder.with_token(session_token.as_str());
        }

        if let Some(endpoint) = server_options.get(AmazonServerOption::Endpoint.as_str()) {
            builder = builder.with_endpoint(endpoint.as_str());
        }

        if let Some(allow_http) = server_options.get(AmazonServerOption::AllowHttp.as_str()) {
            if allow_http == "true" {
                builder = builder.with_allow_http(true);
            }
        }

        if let Some(skip_signature) = server_options.get(AmazonServerOption::SkipSignature.as_str())
        {
            if skip_signature == "true" {
                builder = builder.with_skip_signature(true);
            }
        }

        Ok(builder.build()?)
    }
}

#[derive(Error, Debug)]
pub enum ObjectStoreError {
    #[error(transparent)]
    OptionsError(#[from] OptionsError),

    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),
}
