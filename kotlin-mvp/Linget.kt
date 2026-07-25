import com.googlecode.lanterna.TerminalSize
import com.googlecode.lanterna.TextColor
import com.googlecode.lanterna.gui2.*
import com.googlecode.lanterna.gui2.BorderLayout.Location
import com.googlecode.lanterna.gui2.table.Table
import com.googlecode.lanterna.screen.TerminalScreen
import com.googlecode.lanterna.terminal.DefaultTerminalFactory

// Package data class
data class Package(
    val name: String,
    val version: String,
    val source: String,
    val status: String,
    val size: String,
    val description: String,
    val stars: String
)

// Task data class
data class Task(
    val packageName: String,
    val action: String,
    var progress: Int,
    var status: String
)

fun main() {
    // Sample data
    val packages = listOf(
        Package("neovim", "0.9.5", "apt", "installed", "15MB", "Modern vim editor", "78.2k"),
        Package("firefox", "124.0.1", "flatpak", "update", "120MB", "Web browser", "245k"),
        Package("vscode", "1.87.2", "flatpak", "installed", "320MB", "VS Code", "158k"),
        Package("docker", "25.0.4", "apt", "update", "150MB", "Container runtime", "12.3k"),
        Package("nodejs", "20.11.1", "apt", "not_installed", "35MB", "JavaScript runtime", "102k"),
        Package("rust-toolchain", "1.77.0", "cargo", "installed", "800MB", "Rust language", "89.4k"),
        Package("alacritty", "0.13.1", "cargo", "installed", "8MB", "GPU terminal", "56.2k"),
        Package("postgresql", "16.2", "apt", "update", "45MB", "SQL database", "12.8k")
    )

    val tasks = mutableListOf<Task>()
    var selectedIndex = 0
    var currentFilter = "all"

    // Setup terminal
    val terminal = DefaultTerminalFactory().createTerminal()
    val screen = TerminalScreen(terminal)
    screen.startScreen()

    // Create GUI
    val panel = Panel(BorderLayout())
    
    // Title
    val titlePanel = Panel(BorderLayout())
    val titleLabel = Label("  LinGet 2.0 - Universal Package Manager  ")
        .setBackgroundColor(TextColor.ANSI.BLUE)
        .setForegroundColor(TextColor.ANSI.WHITE)
    titlePanel.addComponent(titleLabel, Location.LEFT)
    panel.addComponent(titlePanel, Location.TOP)

    // Table for packages
    val table = Table<String>("Status", "Name", "Version", "Source", "Size")
    
    fun getFiltered() = if (currentFilter == "all") packages else packages.filter { it.source == currentFilter }
    
    fun refreshTable() {
        table.tableModel.rows.clear()
        val filtered = getFiltered()
        
        filtered.forEach { pkg ->
            val status = when (pkg.status) {
                "installed" -> "✓"
                "update" -> "⬆"
                else -> "○"
            }
            table.tableModel.addRow(status, pkg.name, pkg.version, pkg.source, pkg.size)
        }
    }
    
    refreshTable()

    // Detail panel
    val detailPanel = Panel(BorderLayout())
    val detailLabel = Label("Select a package to view details")
        .setPreferredSize(TerminalSize(40, 15))
    detailPanel.addComponent(detailLabel, Location.CENTER)

    fun refreshDetail() {
        val filtered = getFiltered()
        if (selectedIndex < filtered.size) {
            val pkg = filtered[selectedIndex]
            val statusStr = when (pkg.status) {
                "installed" -> "✓ INSTALLED"
                "update" -> "⬆ UPDATE AVAILABLE"
                else -> "○ NOT INSTALLED"
            }
            detailLabel.text = """
                ${pkg.name} ${pkg.version} [${pkg.source.uppercase()}]
                
                $statusStr
                Size: ${pkg.size}
                Stars: ${pkg.stars}
                
                ${pkg.description}
                
                Press i=install, u=update, r=remove
            """.trimIndent()
        }
    }

    table.setSelectAction {
        selectedIndex = table.selectedRow
        refreshDetail()
    }

    // Queue panel
    val queueLabel = Label("Queue empty - press 'n' for demo tasks")
        .setPreferredSize(TerminalSize(50, 5))

    fun refreshQueue() {
        if (tasks.isEmpty()) {
            queueLabel.text = "Queue empty - press 'n' for demo tasks"
        } else {
            val sb = StringBuilder("Task Queue:\n")
            tasks.forEach { task ->
                val icon = when (task.status) {
                    "done" -> "✓"
                    "running" -> "▶"
                    else -> "○"
                }
                val filled = task.progress / 5
                val bar = "█".repeat(filled) + "░".repeat(20 - filled)
                sb.append("$icon ${task.packageName} [$bar] ${task.progress}%\n")
            }
            queueLabel.text = sb.toString()
        }
    }

    // Filter buttons
    val tabsPanel = Panel(LinearLayout(Direction.HORIZONTAL))
    tabsPanel.addComponent(Button("[1] ALL") {
        currentFilter = "all"
        refreshTable()
    })
    tabsPanel.addComponent(Button("[2] APT") {
        currentFilter = "apt"
        refreshTable()
    })
    tabsPanel.addComponent(Button("[3] FLATPAK") {
        currentFilter = "flatpak"
        refreshTable()
    })
    tabsPanel.addComponent(Button("[4] CARGO") {
        currentFilter = "cargo"
        refreshTable()
    })
    panel.addComponent(tabsPanel, Location.TOP)

    // Center panel
    val centerPanel = Panel(BorderLayout())
    centerPanel.addComponent(table, Location.CENTER)
    centerPanel.addComponent(detailPanel, Location.RIGHT)
    panel.addComponent(centerPanel, Location.CENTER)

    // Queue at bottom
    panel.addComponent(queueLabel, Location.BOTTOM)

    // Action buttons
    val actionPanel = Panel(LinearLayout(Direction.HORIZONTAL))
    actionPanel.addComponent(Button("[i] Install") {
        val filtered = getFiltered()
        if (selectedIndex < filtered.size) {
            tasks.add(Task(filtered[selectedIndex].name, "install", 0, "queued"))
            refreshQueue()
        }
    })
    actionPanel.addComponent(Button("[u] Update") {
        val filtered = getFiltered()
        if (selectedIndex < filtered.size) {
            tasks.add(Task(filtered[selectedIndex].name, "update", 0, "queued"))
            refreshQueue()
        }
    })
    actionPanel.addComponent(Button("[r] Remove") {
        val filtered = getFiltered()
        if (selectedIndex < filtered.size) {
            tasks.add(Task(filtered[selectedIndex].name, "remove", 0, "queued"))
            refreshQueue()
        }
    })
    actionPanel.addComponent(Button("[n] Demo") {
        tasks.add(Task("rust-analyzer", "install", 0, "queued"))
        tasks.add(Task("exa", "install", 0, "queued"))
        tasks.add(Task("bat", "update", 0, "queued"))
        refreshQueue()
    })
    actionPanel.addComponent(Button("[q] Quit") {
        screen.stopScreen()
        System.exit(0)
    })
    panel.addComponent(actionPanel, Location.BOTTOM)

    // Window
    val window = BasicWindow("LinGet")
    window.component = panel
    window.setHints(listOf(Window.Hint.FULL_SCREEN))

    // GUI
    val gui = MultiWindowTextGUI(screen)
    gui.addWindow(window)

    // Background task progress
    val running = true
    Thread {
        while (running) {
            Thread.sleep(100)
            var changed = false
            tasks.forEach { task ->
                if (task.status == "running") {
                    task.progress += 5
                    if (task.progress >= 100) {
                        task.progress = 100
                        task.status = "done"
                    }
                    changed = true
                } else if (task.status == "queued") {
                    task.status = "running"
                    changed = true
                }
            }
            if (changed) {
                // Update on UI thread would go here
            }
        }
    }.start()

    gui.waitForWindowToClose(window)
    screen.stopScreen()
}
