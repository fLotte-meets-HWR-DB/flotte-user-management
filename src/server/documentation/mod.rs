//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

use schemars::JsonSchema;
use std::collections::HashMap;

use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

pub struct RESTDocumentation {
    paths: HashMap<String, String>,
    base_path: String,
}

impl RESTDocumentation {
    pub fn new(base_path: &str) -> Self {
        Self {
            paths: HashMap::new(),
            base_path: base_path.to_string(),
        }
    }

    pub fn get(&self, path: String) -> String {
        log::trace!("Rendering help for {}.", path);
        format!(
            "<html><head><style type='text/css'>{}</style></head><body>{}</body></html>",
            include_str!("style.css"),
            self.paths.get(&path).unwrap_or(&self.landing())
        )
    }

    fn landing(&self) -> String {
        let types = self.paths.keys().fold("".to_string(), |a, b| {
            format!("{}<br><a href='{}?path={2}'>{2}</a>", a, self.base_path, b)
        });

        format!("<h1>Paths</h1><br>{}", types)
    }

    pub fn add_path<I: JsonSchema, O: JsonSchema>(
        &mut self,
        path: &str,
        method: &str,
        description: &str,
    ) -> Result<(), serde_json::error::Error> {
        let input_schema = schema_for!(I);
        let output_schema = schema_for!(O);

        let input_json = highlight_json(serde_json::to_string_pretty(&input_schema)?);
        let output_json = highlight_json(serde_json::to_string_pretty(&output_schema)?);
        let content = format!(
            "\
            <a href={}>Back</a>
            <h1>{}: {}</h1>
            <p>{}</p>
            <h2>Input</h2>
            <code>{}</code>
            <h2>Output</h2>
            <code>{}</code>
        ",
            self.base_path, method, path, description, input_json, output_json
        );
        self.paths.insert(path.to_string(), content);
        Ok(())
    }
}

fn highlight_json(input: String) -> String {
    lazy_static::lazy_static! { static ref PS: SyntaxSet = SyntaxSet::load_defaults_nonewlines(); }
    lazy_static::lazy_static! { static ref TS: ThemeSet = ThemeSet::load_defaults(); }

    highlighted_html_for_string(
        input.as_str(),
        &PS,
        PS.find_syntax_by_token("json").unwrap(),
        &TS.themes["InspiredGitHub"],
    )
}
