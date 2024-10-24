use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use clipboard::{ClipboardContext, ClipboardProvider};

#[derive(Deserialize)]
struct Config {
    exclude_modules: HashSet<String>,
    exclude_start_phrases: Vec<String>,
    output_file: String,
}

fn main() -> io::Result<()> {
    // Подсказка пользователю
    println!("Скопируйте текстовый лог в буфер обмена и нажмите Enter, чтобы продолжить...");

    // Ждем ввода от пользователя для продолжения
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Получаем текст из буфера обмена с помощью утилиты `xclip` или `pbpaste` (зависит от ОС)
    let text = match get_clipboard_content() {
        Some(content) => content,
        None => {
            eprintln!("Не удалось получить данные из буфера обмена.");
            return Ok(());
        }
    };

    // Определяем путь к исполняемому файлу и используем его для нахождения конфигурационного файла
    let executable_path = env::current_exe()?;
    let config_file_path = executable_path
        .parent()
        .expect("Не удалось определить директорию исполняемого файла")
        .join("mint-log-filter.json");

    // Читаем конфигурационный файл
    let config_data = fs::read_to_string(config_file_path)
        .expect("Не удалось прочитать конфигурационный файл");
    let config: Config = serde_json::from_str(&config_data)
        .expect("Ошибка при десериализации конфигурационного файла");

    // Создаем регулярные выражения
    let log_line_re = Regex::new(r"(\d{2}:\d{2}:\d{2}\.\d{3}) \w+ \[(.*?)\] - (.*)").unwrap();
    let end_brackets_re = Regex::new(r"\s*\[\[.*?\]\]\s*$").unwrap();

    let mut filtered_lines = Vec::new();
    let mut previous_module = String::new();

    // Обрабатываем текст
    for line in text.lines() {
        // Проверяем строку на соответствие регулярному выражению
        if let Some(captures) = log_line_re.captures(line) {
            let timestamp = captures.get(1).map_or("", |m| m.as_str());
            let module = captures.get(2).map_or("", |m| m.as_str()).trim();
            let message = captures.get(3).map_or("", |m| m.as_str()).trim();

            // Пропускаем строки, если модуль входит в список исключений
            if config.exclude_modules.contains(module) {
                continue;
            }

            // Пропускаем строки, если сообщение начинается с одной из фраз в списке исключений
            if config
                .exclude_start_phrases
                .iter()
                .any(|phrase| message.starts_with(phrase))
            {
                continue;
            }

            // Удаляем текст в двойных квадратных скобках в конце сообщения
            let cleaned_message = end_brackets_re.replace(message, "").to_string();

            // Добавляем пустую строку между разными модулями
            if !previous_module.is_empty() && previous_module != module {
                filtered_lines.push(String::new());
            }

            // Добавляем оставшуюся строку в список
            filtered_lines.push(format!("{timestamp} [{:<25}] - {cleaned_message}", module));
            previous_module = module.to_string();
        }
    }

    // Записываем результат в файл, путь к которому указан в конфигурации
    let output_file_path = PathBuf::from(&config.output_file);
    let mut output_file = File::create(output_file_path)?;
    for line in filtered_lines {
        writeln!(output_file, "{}", line)?;
    }

    println!("Фильтрованный лог сохранен в файл '{}'.", config.output_file);
    Ok(())
}

// Функция для получения содержимого буфера обмена
fn get_clipboard_content() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        // Используем xclip для Linux
        let output = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg("-o")
            .output()
            .ok()?;
        if output.stdout.is_empty() {
            return None;
        }
        return String::from_utf8(output.stdout).ok();
    }

    #[cfg(target_os = "macos")]
    {
        // Используем pbpaste для macOS
        let output = Command::new("pbpaste").output().ok()?;
        if output.stdout.is_empty() {
            return None;
        }
        return String::from_utf8(output.stdout).ok();
    }

    #[cfg(target_os = "windows")]
    {
        // Используем библиотеку clipboard для Windows
        let mut ctx: ClipboardContext = ClipboardProvider::new().ok()?;
        return ctx.get_contents().ok();
    }
}
