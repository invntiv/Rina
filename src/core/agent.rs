use rig::agent::Agent as RigAgent;
use rig::providers::anthropic::completion::CompletionModel;
use rig::providers::anthropic::{self, CLAUDE_3_HAIKU};
use rig::completion::Prompt;
use rand::{self};
use serde_json::json;

use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
}; 

use teloxide::prelude::*;

pub struct Agent {
    agent: RigAgent<CompletionModel>,
    anthropic_api_key: String,
    pub prompt: String,
}

#[derive(Debug, PartialEq)]
pub enum ResponseDecision {
    Respond,
    Ignore,
}

impl Agent {
    pub fn new(anthropic_api_key: &str, prompt: &str) -> Self {
        let client = anthropic::ClientBuilder::new(anthropic_api_key).build();
        let rng = rand::thread_rng();
        let temperature = 0.9; // Higher temperature for more variety

        let agent = client
            .agent(CLAUDE_3_HAIKU)
            .preamble(prompt)
            .temperature(temperature)
            .max_tokens(4096)
            .build();
        Agent { 
            agent,
            anthropic_api_key: anthropic_api_key.to_string(),
            prompt: prompt.to_string(),
        }
    }

    pub async fn should_respond(&self, tweet: &str) -> Result<ResponseDecision, anyhow::Error> {
        let prompt = format!(
            "Tweet: {tweet}\n\
            Task: Reply [RESPOND] or [IGNORE] based on:\n\
            [RESPOND] if:\n\
            - Direct mention/address\n\
            - Contains question\n\
            - Contains command/request\n\
            [IGNORE] if:\n\
            - Unrelated content\n\
            - Spam/nonsensical\n\
            Answer:"
        );
        let response = self.agent.prompt(&prompt).await?;
        let response = response.to_uppercase();
        Ok(if response.contains("[RESPOND]") {
            ResponseDecision::Respond
        } else {
            ResponseDecision::Ignore
        })
    }

    pub async fn generate_reply(&self, tweet: &str) -> Result<String, anyhow::Error> {
        let prompt = format!(
            "Task: Generate a post/reply in your voice, style and perspective while using this as context:\n\
            Current Post: '{}'\n\
            Generate a brief, single response that:\n\
            - Uses all lowercase\n\
            - Avoids punctuation\n\
            - Is direct and possibly sarcastic\n\
            - Stays under 280 characters\n\
            Write only the response text, nothing else:",
            tweet
        );
        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }

    pub async fn generate_post(&self) -> Result<String, anyhow::Error> {
        let prompt = r#"Write a 1-3 sentence post that would be engaging to readers. Your response should be the EXACT text of the tweet only, with no introductions, meta-commentary, or explanations.

            Requirements:
            - Stay under 280 characters
            - No emojis
            - No hashtags
            - No questions
            - Brief, concise statements only
            - Focus on personal experiences, observations, or thoughts
            - Write ONLY THE TWEET TEXT with no additional words or commentary"#;
        
        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }

    pub async fn generate_generic_fud(&self, intro: &str, reason: &str, closing: &str) -> Result<String, anyhow::Error> {
        let prompt = format!(
            "{}\n\nTask: Generate a toxic, cynical comment incorporating these elements:\n\
            Intro: {}\n\
            FUD Reason: {}\n\
            Closing: {}\n\n\
            Requirements:\n\
            - Be extremely sarcastic and cynical\n\
            - Incorporate the provided intro, reason, and closing creatively\n\
            - Don't include a ticker
            - Stay under 280 characters\n\
            - Use all lowercase except for token symbols\n\
            - Be creative with metaphors about scams, rugpulls, or dev behavior\n\
            - Avoid hashtags\n\
            - No emojis (they will be added later)\n\
            Write ONLY the tweet text with no additional commentary:",
            self.prompt,
            intro,
            reason,
            closing
        );

        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }

    pub async fn generate_editorialized_fud(&self, token_info: &str) -> Result<String, anyhow::Error> {
        let prompt = format!(
            "{}\n\nTask: Generate a toxic, cynical commentary about this token:\n{}\n\
            Requirements:\n\
            - Be extremely sarcastic and cynical\n\
            - Always use proper token symbol from the info\n\
            - Use ONLY the actual numbers provided in the token info (liquidity, market cap)\n\
            - Don't mention the price\n\
            - Use information from SOLANA chain. Do not mention BNB tokens.
            - If no numbers are available, focus on qualitative criticism instead\n\
            - Never make up specific numbers - if you need a number, use vague terms like 'countless' or 'zero'\n\
            - Be creative with metaphors about scams, rugpulls, or dev behavior\n\
            - Stay under 280 characters\n\
            - Use all lowercase except for token symbols\n\
            - Avoid hashtags\n\
            - Here are some additional examples of FUD:\n\
                'Dev wallet holds 99.9% of supply (trust me bro)'\n\
                'Hawk Tuah team behind this.'\n\
                'Dev is Jewish. Fading.'\n\
                'Website looks like it was made by a retarded 5-year-old'\n\
                Telegram admin can't spell for shit.'\n\
                'My wife's boyfriend says it's a rugpull'\n\
                'Chart looks like the Titanic's final moments'\n\
                'Devs are probably just three raccoons in a trenchcoat'\n\
                'Obvious scam.'\n\
                'Federal Honeypot.'\n\
                'This one is just clearly NGMI and if you buy it you deserve to be poor.'\n\
                'Smart contract security looks like Swiss cheese'\n\
                'Marketing strategy is just paying Nigerians $1 to spam rocket emojis'\n\
                'Good coin for a 10% gain (waste of time).'\n\
                'Just put the fries in the bag, you'd make more money that way.'\n\
                'Reporting dev to the SEC.'\n\
            Write ONLY the tweet text with no additional commentary:",
            self.prompt,
            token_info,
        );

        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }   

    pub async fn generate_image(&self) -> Result<String, anyhow::Error> {
        let client = reqwest::Client::builder().build()?;
        dotenv::dotenv().ok();
        let heuris_api = env::var("HEURIS_API")
            .map_err(|_| anyhow::anyhow!("HEURIS_API not found in environment"))?;
        let base_prompt = env::var("IMAGE_PROMPT")
            .map_err(|_| anyhow::anyhow!("IMAGE_PROMPT not found in environment"))?;
        let deadline = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 300;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", heuris_api).parse()?);
        headers.insert("Content-Type", "application/json".parse()?);

        let body = json!({
            "model_input": {
                "SD": {
                    "width": 1024,
                    "height": 1024,
                    "prompt": format!("{}", base_prompt),
                    "neg_prompt": "worst quality, bad quality, umbrella, blurry face, anime, illustration",
                    "num_iterations": 22,
                    "guidance_scale": 7.5
                }
            },
            "model_id": "BluePencilRealistic",
            "deadline": deadline,
            "priority": 1,
            "job_id": format!("job_{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
        });

        
        let request = client
            .request(
                reqwest::Method::POST,
                "http://sequencer.heurist.xyz/submit_job",
            )
            .headers(headers)
            .json(&body);

        let response = request.send().await?;
        let body = response.text().await?;
        Ok(body.trim_matches('"').to_string())
    }

    pub async fn prepare_image_for_tweet(&self, image_url: &str) -> Result<Vec<u8>, anyhow::Error> {
        let client = reqwest::Client::new();
        let response = client.get(image_url).send().await?;

        Ok(response.bytes().await?.to_vec())
    }

    // pub async fn handle_telegram_message(&self, bot: &Bot) {
    //     let client = anthropic::ClientBuilder::new(&self.anthropic_api_key).build();
    //     let bot = bot.clone();
    //     let agent_prompt = self.prompt.clone();
    //     teloxide::repl(bot, move |bot: Bot, msg: Message| {
    //         let agent = client
    //             .agent(CLAUDE_3_HAIKU)
    //             .preamble(&agent_prompt)
    //             .temperature(0.5)
    //             .max_tokens(4096)
    //             .build();
    //         async move {
    //             if let Some(text) = msg.text() {
    //                 let should_respond = msg.chat.is_private() || text.contains("@rina_rig_bot");
                    
    //                 if should_respond {
    //                     let combined_prompt = format!(
    //                         "Task: Generate a conversational reply to this Telegram message while using this as context:\n\
    //                         Message: '{}'\n\
    //                         Generate a natural response that:\n\
    //                         - Is friendly and conversational\n\
    //                         - Can use normal punctuation and capitalization\n\
    //                         - May include emojis when appropriate\n\
    //                         - Maintains a helpful and engaging tone\n\
    //                         - Keeps responses concise but not artificially limited\n\
    //                         Write only the response text, nothing else:",
    //                         text
    //                     );
    //                     let response = agent
    //                         .prompt(&combined_prompt)
    //                         .await
    //                         .expect("Error generating the response");
    //                     println!("Telegram response: {}", response);
    //                     bot.send_message(msg.chat.id, response).await?;
    //                 }
    //             }
    //             Ok(())
    //         }
    //     })
    //     .await;
    // }
}

