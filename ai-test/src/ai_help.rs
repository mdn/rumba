use std::{
    iter,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use anyhow::Error;
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{ChatCompletionRequestMessage, ChatCompletionResponseMessage, Role::User},
};
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use rumba::{
    ai::help::{prepare_ai_help_req, AIHelpRequest},
    db,
    gemini::{self, GenerateContentRequest},
    settings::SETTINGS,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::prompts;

#[derive(Serialize, Deserialize)]
pub struct Storage {
    pub req: AIHelpRequest,
    pub res: Option<ChatCompletionResponseMessage>,
}

const MD_DELIM: &str = "\n---\n---\n";

fn msg_to_md(msg: &ChatCompletionRequestMessage) -> String {
    let role = &msg.role;
    let content = msg.content.as_deref().unwrap_or_default();
    format!("{role}:{MD_DELIM}{content}")
}

impl Storage {
    pub fn to_md(&self) -> String {
        let docs = self
            .req
            .refs
            .iter()
            .map(|r| format!("[{}]({})", r.title, r.url))
            .join("\n");
        let res = if let Some(res) = &self.res {
            let res_content = res.content.as_deref().unwrap_or_default();
            let res_role = &res.role;
            format!("{res_role}:{MD_DELIM}{res_content}")
        } else {
            "**no response**".to_string()
        };
        self.req
            .req
            .messages
            .iter()
            .map(msg_to_md)
            .chain(iter::once(res))
            .chain(iter::once(docs))
            .join(MD_DELIM)
    }
}

pub async fn ai_help_all(
    path: Option<impl AsRef<Path>>,
    out: impl AsRef<Path>,
    no_subscription: bool,
) -> Result<(), Error> {
    let out = &out;
    std::fs::create_dir_all(out)?;
    let supabase_pool = &{
        let uri = SETTINGS.db.supabase_uri.as_ref().expect("no supabase");
        db::establish_supa_connection(uri).await
    };

    let openai_client = &Arc::new(async_openai::Client::with_config(
        OpenAIConfig::new().with_api_key(&SETTINGS.ai.as_ref().expect("no ai settings").api_key),
    ));

    let gemini_client = &Arc::new(
        SETTINGS
            .ai
            .as_ref()
            .map(|c| {
                let mut config = gemini::GeminiConfig::new();
                if let Some(api_key) = &c.gemini_api_key {
                    config = config.with_api_key(api_key);
                }
                if let Some(model) = &c.gemini_model {
                    config = config.with_model(model);
                }
                gemini::GeminiClient::with_config(config)
            })
            .unwrap(),
    );

    let prompts = prompts::read(path)?;
    let total_samples = prompts.len();
    let before = Instant::now();
    stream::iter(
        prompts
            .into_iter()
            .enumerate()
            .filter(|(i, _val)| std::fs::metadata(json_path(out, *i)).is_err()),
    )
    .map(Ok::<(usize, Vec<String>), Error>)
    .try_for_each_concurrent(10, |(i, prompts)| async move {
        println!("processing: {:0>2}", i);
        let json_out = json_path(out, i);
        let md_out = md_path(out, i);
        let messages = prompts
            .into_iter()
            .map(|prompt| ChatCompletionRequestMessage {
                role: User,
                content: Some(prompt),
                name: None,
                function_call: None,
            })
            .collect();
        if let Some(req) =
            prepare_ai_help_req(openai_client, supabase_pool, !no_subscription, messages).await?
        {
            let gemini_req = GenerateContentRequest::from(req.req.clone());

            let mut gemini_res = gemini_client
                .create(gemini_req.clone())
                .await
                .map_err(|_| OpenAIError::StreamError(String::new()))?;

            let res: Option<ChatCompletionResponseMessage> = gemini_res
                .candidates
                .pop()
                .and_then(|res| res.content)
                .map(|content| -> ChatCompletionResponseMessage { content.into() });
            let storage = Storage {
                req: AIHelpRequest {
                    req: gemini_req.clone().into(),
                    refs: req.refs,
                },
                res,
            };
            println!("writing: {}", json_out.display());
            fs::write(json_out, serde_json::to_vec_pretty(&storage)?).await?;
            println!("writing: {}", md_out.display());
            fs::write(md_out, storage.to_md().as_bytes()).await?;
        }
        Ok(())
    })
    .await?;
    let after = Instant::now();
    println!(
        "Tested {} prompts in {} seconds",
        total_samples,
        after.duration_since(before).as_secs()
    );
    Ok(())
}

fn json_path(out: &impl AsRef<Path>, i: usize) -> PathBuf {
    PathBuf::from(out.as_ref()).join(format!("{:0>2}.json", i))
}

fn md_path(out: &impl AsRef<Path>, i: usize) -> PathBuf {
    PathBuf::from(out.as_ref()).join(format!("{:0>2}.md", i))
}
