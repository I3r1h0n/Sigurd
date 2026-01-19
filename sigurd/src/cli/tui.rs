use std::io::{self, Write};
use terminal_size::{terminal_size, Width, Height};
use crossterm::{
    ExecutableCommand, 
    cursor::{Hide, MoveTo, Show}, 
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode}
};

static LOGO_ART: &'static str = r#"

  _________.__                        .___
 /   _____/|__| ____  __ _________  __| _/
 \_____  \ |  |/ ___\|  |  \_  __ \/ __ | 
 /        \|  / /_/  >  |  /|  | \/ /_/ | 
/_______  /|__\___  /|____/ |__|  \____ | 
        \/   /_____/                   \/     
"#;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

pub struct Tui {
    stdout: io::Stdout,
    current_content_row: u16,
    header_height: u16,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        let stdout = io::stdout();
        let logo_lines = LOGO_ART.lines().count() as u16;
        let info_lines = 2; 
        let spacing = 2; 
        let header_height = logo_lines + info_lines + spacing + 1; 
        
        Ok(Self { 
            stdout,
            current_content_row: header_height,
            header_height,
        })
    }
    
    pub fn clear(&mut self) -> io::Result<()> {
        self.stdout.execute(Clear(ClearType::All))?;
        self.stdout.execute(MoveTo(0, 0))?;
        self.current_content_row = self.header_height;
        self.stdout.flush()
    }
    
    pub fn clear_content(&mut self) -> io::Result<()> {
        if let Some((_width, height)) = self.get_terminal_size() {
            for row in self.header_height..height {
                self.stdout.execute(MoveTo(0, row))?;
                self.stdout.execute(Clear(ClearType::UntilNewLine))?;
            }
        }
        self.current_content_row = self.header_height;
        self.stdout.execute(MoveTo(0, self.header_height))?;
        self.stdout.flush()
    }
    
    fn get_terminal_size(&self) -> Option<(u16, u16)> {
        terminal_size().map(|(Width(w), Height(h))| (w, h))
    }

    fn calculate_center_x(&self, text_width: usize) -> Option<u16> {
        self.get_terminal_size().map(|(term_width, _)| {
            let term_width = term_width as usize;
            if text_width >= term_width {
                0 
            } else {
                ((term_width - text_width) / 2) as u16
            }
        })
    }
    
    fn print_centered(&mut self, text: &str, row: u16) -> io::Result<()> {
        let text_width = text.len();
        if let Some(center_x) = self.calculate_center_x(text_width) {
            self.stdout.execute(MoveTo(center_x, row))?;
            write!(&mut self.stdout, "{}", text)?;
        } else {
            self.stdout.execute(MoveTo(0, row))?;
            write!(&mut self.stdout, "{}", text)?;
        }
        self.stdout.flush()
    }

    fn print_ascii_art(&mut self, art: &str, start_row: u16) -> io::Result<()> {
        let lines: Vec<&str> = art.lines().collect(); 
        let max_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
        
        if let Some(center_x) = self.calculate_center_x(max_width) {
            for (i, line) in lines.iter().enumerate() {
                let row = start_row + i as u16;
                self.stdout.execute(MoveTo(center_x, row))?;
                write!(&mut self.stdout, "{}", line)?;
            }
        } else {
            for (i, line) in lines.iter().enumerate() {
                let row = start_row + i as u16;
                self.stdout.execute(MoveTo(0, row))?;
                write!(&mut self.stdout, "{}", line)?;
            }
        }
        
        self.stdout.flush()
    }
    
    pub fn print_header(&mut self) -> io::Result<()> {
        self.clear()?;
        let logo_lines: Vec<&str> = LOGO_ART.lines().collect();
        let logo_height = logo_lines.len() as u16;
        
        let start_row = 0;
        
        self.print_ascii_art(LOGO_ART, start_row)?;
        
        let mut info_start_row = start_row + logo_height + 2;
        let info_texts = vec![
            format!("{} version {}", APP_NAME, APP_VERSION),
            format!("By: {}", APP_AUTHORS),
        ];

        for line in info_texts {
            self.print_centered(line.as_str(), info_start_row)?;
            info_start_row += 1;
        }
        
        if let Some((width, _)) = self.get_terminal_size() {
            self.stdout.execute(MoveTo(0, self.header_height - 1))?;
            write!(&mut self.stdout, "{}", "─".repeat(width as usize))?;
        }
        
        self.current_content_row = self.header_height;
        self.stdout.execute(MoveTo(0, self.current_content_row))?;
        
        Ok(())
    }
    
    pub fn println(&mut self, text: &str) -> io::Result<()> {
        self.stdout.execute(MoveTo(0, self.current_content_row))?;
        write!(&mut self.stdout, "{}", text)?;
        self.current_content_row += 1;
        self.stdout.flush()
    }
    
    pub fn print(&mut self, text: &str) -> io::Result<()> {
        self.stdout.execute(MoveTo(0, self.current_content_row))?;
        write!(&mut self.stdout, "{}", text)?;
        self.stdout.flush()
    }
    
    pub fn get_input(&mut self, prompt: &str) -> io::Result<String> {
        let prompt_row = self.current_content_row;
        self.print(&format!("{} ", prompt))?;
        self.stdout.flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        // Clear the prompt line
        self.stdout.execute(MoveTo(0, prompt_row))?;
        self.stdout.execute(Clear(ClearType::UntilNewLine))?;
        self.current_content_row = prompt_row;

        Ok(input.trim().to_string())
    }

    pub fn get_yes_no(&mut self, question: &str) -> io::Result<bool> {
        loop {
            let prompt_row = self.current_content_row;
            self.print(&format!("{} (y/n): ", question))?;
            self.stdout.flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => {
                    // Clear the prompt line
                    self.stdout.execute(MoveTo(0, prompt_row))?;
                    self.stdout.execute(Clear(ClearType::UntilNewLine))?;
                    self.current_content_row = prompt_row;
                    return Ok(true);
                }
                "n" | "no" => {
                    self.stdout.execute(MoveTo(0, prompt_row))?;
                    self.stdout.execute(Clear(ClearType::UntilNewLine))?;
                    self.current_content_row = prompt_row;
                    return Ok(false);
                }
                _ => {
                    // Clear the invalid input line
                    self.stdout.execute(MoveTo(0, prompt_row))?;
                    self.stdout.execute(Clear(ClearType::UntilNewLine))?;
                    self.println("Please answer with 'y' or 'n'.")?;
                }
            }
        }
    }
    
    pub fn select_from_list(&mut self, prompt: &str, items: &[String]) -> io::Result<Option<usize>> {
        if items.is_empty() {
            return Ok(None);
        }

        let prompt_row = self.current_content_row;
        self.println(prompt)?;
        let list_start_row = self.current_content_row;

        enable_raw_mode()?;
        self.stdout.execute(Hide)?;
        self.stdout.flush()?;

        // Flush pending events
        while event::poll(std::time::Duration::from_millis(0))? {
            let _ = event::read();
        }

        let mut selected: usize = 0;

        loop {
            for (i, item) in items.iter().enumerate() {
                let row = list_start_row + i as u16;
                self.stdout.execute(MoveTo(0, row))?;
                self.stdout.execute(Clear(ClearType::UntilNewLine))?;

                if i == selected {
                    write!(&mut self.stdout, "> {}", item)?;
                } else {
                    write!(&mut self.stdout, "  {}", item)?;
                }
            }
            self.stdout.flush()?;

            if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                if kind != KeyEventKind::Press {
                    continue;
                }

                match code {
                    KeyCode::Up => {
                        selected = if selected == 0 {
                            items.len() - 1
                        } else {
                            selected - 1
                        };
                    }
                    KeyCode::Down => {
                        selected = (selected + 1) % items.len();
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        disable_raw_mode()?;
        self.stdout.execute(Show)?;

        for row in prompt_row..(list_start_row + items.len() as u16) {
            self.stdout.execute(MoveTo(0, row))?;
            self.stdout.execute(Clear(ClearType::UntilNewLine))?;
        }

        self.current_content_row = prompt_row;

        Ok(Some(selected))
    }
}