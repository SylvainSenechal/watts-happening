# Watts Happening? ⚡🚴

A Rust-powered cycling performance analytics dashboard with D3.js visualizations, automatically syncing with Strava data.

## 🎯 Project Goals

Analyze and visualize cycling performance metrics over time, including:
- Heart rate zones and trends
- Power output analysis and power curves
- Cadence (RPM) patterns
- Speed and elevation profiles
- Performance evolution and personal records
- Correlations between different metrics

## 🏗️ Architecture

### Backend (Rust)
- Fetch activities from Strava API
- Parse and process activity data (GPX, TCX, FIT formats)
- Generate JSON datasets for visualization
- Handle authentication and API rate limiting

### Frontend (D3.js + HTML/CSS)
- Interactive time-series charts
- Power curve visualizations
- Heart rate zone distributions
- Performance dashboards
- Hosted on GitHub Pages

### CI/CD (GitHub Actions)
- Scheduled job to pull latest Strava data
- Automated data processing pipeline
- Deploy updated visualizations to GitHub Pages

## 📁 Project Structure

```
watts-happening/
├── backend/           # Rust API client and data processor
│   ├── src/
│   │   ├── main.rs
│   │   ├── strava/    # Strava API integration
│   │   ├── parser/    # Activity file parsing
│   │   └── export/    # JSON data generation
│   └── Cargo.toml
├── frontend/          # Static site with D3 visualizations
│   ├── index.html
│   ├── css/
│   ├── js/
│   │   └── viz/       # D3 visualization modules
│   └── data/          # Generated JSON data
├── .github/
│   └── workflows/
│       └── sync-data.yml
└── README.md
```

## 🚀 Getting Started

### Prerequisites
- Rust (install via [rustup](https://rustup.rs/))
- Strava API credentials
- GitHub account for hosting

### Strava API Setup

1. Create a Strava API Application:
   - Go to https://www.strava.com/settings/api
   - Create a new application
   - Note your **Client ID** and **Client Secret**
   - Set Authorization Callback Domain (for local testing: `localhost`)

2. Get Access Token:
   - Follow OAuth 2.0 flow to authorize your app
   - Store refresh token securely for automated access

3. Add secrets to GitHub repository:
   ```
   STRAVA_CLIENT_ID
   STRAVA_CLIENT_SECRET
   STRAVA_REFRESH_TOKEN
   ```

### Development

```bash
# Backend
cd backend
cargo build
cargo run

# Frontend (serve locally)
cd frontend
python3 -m http.server 8000
# Visit http://localhost:8000
```

## 📊 Data Flow

1. **GitHub Actions** triggers on schedule (e.g., daily)
2. **Rust backend** authenticates with Strava API
3. Fetches recent activities and detailed streams (heart rate, power, cadence)
4. Processes and aggregates data
5. Exports JSON to `frontend/data/`
6. **GitHub Pages** serves updated visualizations

## 🎨 Planned Visualizations

- [ ] Performance timeline (all metrics over time)
- [ ] Heart rate zone distribution
- [ ] Power duration curve (PDC)
- [ ] Weekly/monthly training load
- [ ] Cadence vs power scatter plots
- [ ] Elevation profiles with overlays
- [ ] Personal records tracker
- [ ] Comparative ride analysis

## 📝 Development Roadmap

### Phase 1: Setup & Data Access
- [ ] Initialize Rust project with dependencies
- [ ] Implement Strava OAuth flow
- [ ] Fetch basic activity list
- [ ] Parse activity details and streams

### Phase 2: Data Processing
- [ ] Create data models for activities
- [ ] Calculate derived metrics (averages, zones, etc.)
- [ ] Export structured JSON

### Phase 3: Visualization
- [ ] Setup static site structure
- [ ] Implement basic D3 charts
- [ ] Add interactivity and filtering
- [ ] Responsive design

### Phase 4: Automation
- [ ] GitHub Actions workflow
- [ ] Automated deployment
- [ ] Error handling and notifications

## 🔐 Security Notes

- Never commit API credentials to the repository
- Use GitHub Secrets for CI/CD
- Store refresh tokens securely
- Consider rate limiting for API requests

## 📚 Resources

- [Strava API Documentation](https://developers.strava.com/docs/reference/)
- [D3.js Documentation](https://d3js.org/)
- [Rust reqwest crate](https://docs.rs/reqwest/)
- [serde for JSON](https://docs.rs/serde/)

## 📄 License

MIT

---

Built with ❤️ and 💪 for data-driven cycling
