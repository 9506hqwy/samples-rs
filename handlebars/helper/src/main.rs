use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use handlebars::{Handlebars, handlebars_helper};
use serde::Serialize;

handlebars_helper!(year: |d: DateTime<Local>| d.year());
handlebars_helper!(month: |d: DateTime<Local>| d.month());
handlebars_helper!(day: |d: DateTime<Local>| d.day());
handlebars_helper!(hour: |d: DateTime<Local>| d.hour());
handlebars_helper!(minute: |d: DateTime<Local>| d.minute());
handlebars_helper!(second: |d: DateTime<Local>| d.second());

handlebars_helper!(utcyear: |d: DateTime<Utc>| d.year());
handlebars_helper!(utcmonth: |d: DateTime<Utc>| d.month());
handlebars_helper!(utcday: |d: DateTime<Utc>| d.day());
handlebars_helper!(utchour: |d: DateTime<Utc>| d.hour());
handlebars_helper!(utcminute: |d: DateTime<Utc>| d.minute());
handlebars_helper!(utcsecond: |d: DateTime<Utc>| d.second());

#[derive(Serialize)]
struct Data {
    now: DateTime<Local>,
    utcnow: DateTime<Utc>,
}

const TEMPLATE: &str = r#"
Local Time:
  Year: {{year now}}
  Month: {{month now}}
  Day: {{day now}}
  Hour: {{hour now}}
  Minute: {{minute now}}
  Second: {{second now}}

UTC Time:
  Year: {{utcyear utcnow}}
  Month: {{utcmonth utcnow}}
  Day: {{utcday utcnow}}
  Hour: {{utchour utcnow}}
  Minute: {{utcminute utcnow}}
  Second: {{utcsecond utcnow}}
"#;

fn main() {
    let mut template = Handlebars::new();
    template.register_helper("year", Box::new(year));
    template.register_helper("month", Box::new(month));
    template.register_helper("day", Box::new(day));
    template.register_helper("hour", Box::new(hour));
    template.register_helper("minute", Box::new(minute));
    template.register_helper("second", Box::new(second));
    template.register_helper("utcyear", Box::new(utcyear));
    template.register_helper("utcmonth", Box::new(utcmonth));
    template.register_helper("utcday", Box::new(utcday));
    template.register_helper("utchour", Box::new(utchour));
    template.register_helper("utcminute", Box::new(utcminute));
    template.register_helper("utcsecond", Box::new(utcsecond));

    let now = Local::now();
    let utcnow = Utc::now();
    let data = Data { now, utcnow };

    let output = template.render_template(TEMPLATE, &data).unwrap();

    println!("{output}");
}
