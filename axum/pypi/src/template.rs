use crate::model;
use minijinja::{Environment, context};

const SIMPLE_TEMPLATE: &str = r#"<Doctype html>
<html>
  <head>
    <meta name="pypi:repository-version" content="1.4">
  </head>
  <body>
  {% for project in projects %}
    <a href="{{ project.name }}/">{{ project.name }}</a><br>
  {% endfor %}
  <body>
</html>
"#;

const PROJECT_TEMPLATE: &str = r#"<Doctype html>
<html>
  <head>
    <meta name="pypi:repository-version" content="1.4">
  </head>
  <body>
  {% for package in packages %}
    <a href="/packages/{{ package.filename }}#{{ hash }}={{ package.hashes[hash] }}">{{ package.filename }}</a><br>
  {% endfor %}
  <body>
</html>
"#;

pub fn render_simple(projects: &[model::SimpleProject]) -> String {
    let mut env = Environment::new();
    env.add_template("simple", SIMPLE_TEMPLATE).unwrap();

    let tmpl = env.get_template("simple").unwrap();
    tmpl.render(context!(projects => projects)).unwrap()
}

pub fn render_project(hash: &str, packages: &[model::ProjectFile]) -> String {
    let mut env = Environment::new();
    env.add_template("project", PROJECT_TEMPLATE).unwrap();

    let tmpl = env.get_template("project").unwrap();
    tmpl.render(context!(hash => hash, packages => packages))
        .unwrap()
}
