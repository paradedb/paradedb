use object_store::aws::{AmazonS3, AmazonS3Builder};
use supabase_wrappers::prelude::require_option;
use thiserror::Error;

use super::options::*;
impl TryFrom<ServerOptions> for AmazonS3 {
    type Error = LakeError;

    fn try_from(options: ServerOptions) -> Result<Self, Self::Error> {
        let ServerOptions(options) = options;

        let url = require_option(AmazonServerOption::Url.as_str(), &options)?;
        let region = require_option(AmazonServerOption::Region.as_str(), &options)?;

        let mut builder = AmazonS3Builder::new().with_url(url).with_region(region);

        if let Some(access_key_id) = options.get(AmazonServerOption::AccessKeyId.as_str()) {
            builder = builder.clone().with_access_key_id(access_key_id.as_str());
        }

        if let Some(secret_access_key) = options.get(AmazonServerOption::SecretAccessKey.as_str()) {
            builder = builder.with_secret_access_key(secret_access_key.as_str());
        }

        if let Some(session_token) = options.get(AmazonServerOption::SessionToken.as_str()) {
            builder = builder.with_token(session_token.as_str());
        }

        if let Some(endpoint) = options.get(AmazonServerOption::Endpoint.as_str()) {
            builder = builder.with_endpoint(endpoint.as_str());
        }

        if let Some(allow_http) = options.get(AmazonServerOption::AllowHttp.as_str()) {
            if allow_http == "true" {
                builder = builder.with_allow_http(true);
            }
        }

        if let Some(skip_signature) = options.get(AmazonServerOption::SkipSignature.as_str()) {
            if skip_signature == "true" {
                builder = builder.with_skip_signature(true);
            }
        }

        Ok(builder.build()?)
    }
}

#[derive(Error, Debug)]
pub enum LakeError {
    #[error(transparent)]
    Option(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),
}
