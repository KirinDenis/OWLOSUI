/*
 * The MIT License
 *
 * Copyright 2026 Slam.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */
package jtunelscope;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.GraphicsDevice;
import java.awt.GraphicsEnvironment;
import java.awt.Point;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionListener;
import java.util.ArrayList;
import java.util.Random;
import javax.swing.JFrame;
import javax.swing.JPanel;
import javax.swing.SwingUtilities;
import javax.swing.Timer;

/**
 *
 * @author Slam
 */
import javax.swing.JFrame;
import javax.swing.SwingUtilities;

public class JTunelScope {

    public static void main(String[] args) {

        SwingUtilities.invokeLater(() -> {

            JFrame frame = new JFrame("Más Allá");

            frame.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
            frame.setResizable(false);

            TunnelPanel panel = new TunnelPanel();

            frame.add(panel);

            frame.pack();
            frame.setLocationRelativeTo(null);
            frame.setVisible(true);

            panel.requestFocusInWindow();

        });

    }

}

class TunnelPanel extends JPanel implements ActionListener, MouseMotionListener {

    // Ubicación de vista del mouse
    private double lookX = 0;
    private double lookY = 0;
    // Lista de rocas
    private final ArrayList<Roca> rocks = new ArrayList<>();
    // Atributos del jugador
    private int score = 0;
    private int level = 1;
    private int health = 100;
    // Clase para generar aleatoriedad
    private final Random random = new Random();
    // Distancia focal (campo de visión)
    private static final double FOCAL = 600;
    // Mitad del lado del túnel
    private static final double SIZE = 200;
    // Profundidad máxima
    private static final double MAX_Z = 5000;
    // Separación entre "anillos"
    private static final double STEP_Z = 200;
    // Velocidad
    private static final double SPEED = 8;
    // Lista de segmentos
    private final ArrayList<Double> slices = new ArrayList<>();
    // Clase para manejo del tiempo y velocidad de aparición en pantalla
    private final Timer timer = new Timer(16, this);

    public TunnelPanel() {
        // Propiedades para el JPanel
        // Obtengo entorno gráfico
        GraphicsEnvironment ge = GraphicsEnvironment.getLocalGraphicsEnvironment();
        // Obtengo el monitor aqui
        GraphicsDevice gd = ge.getDefaultScreenDevice();
        // Si soporta pantalla completa asigno resolución máxima
        // sino una por defecto
        if (gd.isFullScreenSupported()) {
            setPreferredSize(new Dimension(gd.getDisplayMode().getWidth(), gd.getDisplayMode().getHeight()));
        } else {
            setPreferredSize(new Dimension(1024, 768));
        }
        // Color negro para fondo
        setBackground(Color.BLACK);
        // Escucho eventos de movimiento del mouse       
        addMouseMotionListener(this);
        // Inicializando el primer nivel
        generarRoca(level * 5);

        // Dibujo segmentos para efecto tunel wireframe
        for (double z = 300; z < MAX_Z; z += STEP_Z) {
            slices.add(z);
        }
        timer.start();                  // Iniciando Timer
    }

    @Override
    protected void paintComponent(Graphics g) {
        super.paintComponent(g);
        // Objeto para pintar en pantalla usando gráficos
        Graphics2D g2 = (Graphics2D) g;
        g2.setColor(Color.WHITE);

        int cx = getWidth() / 2;
        int cy = getHeight() / 2;

        // Rectángulos del túnel
        for (double z : slices) {
            Point p1 = proyectar(-SIZE, -SIZE, z, cx, cy);
            Point p2 = proyectar(SIZE, -SIZE, z, cx, cy);
            Point p3 = proyectar(SIZE, SIZE, z, cx, cy);
            Point p4 = proyectar(-SIZE, SIZE, z, cx, cy);

            g2.drawLine(p1.x, p1.y, p2.x, p2.y);
            g2.drawLine(p2.x, p2.y, p3.x, p3.y);
            g2.drawLine(p3.x, p3.y, p4.x, p4.y);
            g2.drawLine(p4.x, p4.y, p1.x, p1.y);
        }

        // Líneas de longitud
        int divisions = 10;

        for (int i = 0; i <= divisions; i++) {
            double x = -SIZE + (2 * SIZE * i / divisions);
            Point a = proyectar(x, -SIZE, 300, cx, cy);
            Point b = proyectar(x, -SIZE, MAX_Z, cx, cy);
            g2.drawLine(a.x, a.y, b.x, b.y);
            a = proyectar(x, SIZE, 300, cx, cy);
            b = proyectar(x, SIZE, MAX_Z, cx, cy);
            g2.drawLine(a.x, a.y, b.x, b.y);
        }

        for (int i = 0; i <= divisions; i++) {
            double y = -SIZE + (2 * SIZE * i / divisions);
            Point a = proyectar(-SIZE, y, 300, cx, cy);
            Point b = proyectar(-SIZE, y, MAX_Z, cx, cy);
            g2.drawLine(a.x, a.y, b.x, b.y);
            a = proyectar(SIZE, y, 300, cx, cy);
            b = proyectar(SIZE, y, MAX_Z, cx, cy);
            g2.drawLine(a.x, a.y, b.x, b.y);
        }

        // Dibujando obstáculos (ROCAS)
        g2.setColor(Color.ORANGE);

        for (Roca r : rocks) {
            Point p = proyectar(r.x, r.y, r.z, cx, cy);
            double scale = FOCAL / r.z;
            int radius = (int) (25 * scale);
            g2.fillOval(p.x - radius, p.y - radius, radius * 2, radius * 2);
        }
        // Los textos para NIVEL, PUNTUACIÓN y SALUD
        g2.setColor(Color.GREEN);
        g2.setFont(new Font("Consolas", Font.BOLD, 18));

        g2.drawString("Nivel : " + level, 20, 30);
        g2.drawString("Puntos: " + score + "/100", 20, 55);
        g2.drawString("Salud : " + health, 20, 80);

        // Lógica para finalizar el juego
        // Pendiente tabla de posiciónes y reiniciar partida
        if (health <= 0) {
            g2.setFont(new Font("Arial", Font.BOLD, 60));
            g2.setColor(Color.RED);
            String txt = "GAME OVER";
            FontMetrics fm = g2.getFontMetrics();
            g2.drawString(txt, (getWidth() - fm.stringWidth(txt)) / 2, getHeight() / 2);
        }
    }

    // Función para proyectar en pantalla (nuestro motor 3D a 2D)
    private Point proyectar(double x, double y, double z, int cx, int cy) {
        double scale = FOCAL / z;
        int sx = (int) (cx + (x - lookX) * scale);
        int sy = (int) (cy + (y - lookY) * scale);
        return new Point(sx, sy);
    }

    //-----------------------------------
    @Override
    public void actionPerformed(ActionEvent e) {

        // Animación del túnel
        for (int i = 0; i < slices.size(); i++) {
            double z = slices.get(i);
            z -= SPEED;
            if (z < 300) {
                z = MAX_Z;
            }
            slices.set(i, z);
        }

        // Movimiento de las rocas
        for (Roca r : rocks) {
            r.z -= SPEED * 2;
            // Colisión cerca del jugador
            if (r.z < 250) {
                Point p = proyectar(r.x, r.y, r.z, getWidth() / 2, getHeight() / 2);
                int dx = p.x - getWidth() / 2;
                int dy = p.y - getHeight() / 2;

                double d = Math.sqrt(dx * dx + dy * dy);

                if (d < 45) {
                    health -= 20;
                    r.z = MAX_Z;
                    r.x = random.nextDouble() * SIZE * 2 - SIZE;
                    r.y = random.nextDouble() * SIZE * 2 - SIZE;
                    continue;
                }
            }

            // La roca pasó al jugador
            if (r.z < 80) {
                score += 10;
                r.z = MAX_Z;
                r.x = random.nextDouble() * SIZE * 2 - SIZE;
                r.y = random.nextDouble() * SIZE * 2 - SIZE;
            }

        }

        // Cambio de nivel
        if (score >= 100) {
            score = 0;
            level++;
            generarRoca(level * 5);
        }

        // Fin del juego
        if (health <= 0) {
            timer.stop();
        }
        // repintar el JPanel para refrescar
        // Nuestro ciclo principal en juegos
        repaint();
    }

    @Override
    public void mouseMoved(MouseEvent e) {
        // Obtengo posición relativa del mouse al centro de la pantalla
        double nx = (e.getX() - getWidth() / 2.0) / (getWidth() / 2.0);
        double ny = (e.getY() - getHeight() / 2.0) / (getHeight() / 2.0);
        // Modifico posición de la vista
        lookX = nx * 120;
        lookY = ny * 120;
    }

    @Override
    public void mouseDragged(MouseEvent e) {
        mouseMoved(e);
    }

    private void generarRoca(int amount) {
        rocks.clear();
        for (int i = 0; i < amount; i++) {
            rocks.add(new Roca(random.nextDouble() * SIZE * 2 - SIZE, random.nextDouble() * SIZE * 2 - SIZE, 500 + random.nextDouble() * MAX_Z));
        }
    }
}

class Roca {

    double x, y, z;
    //boolean counted = false;

    Roca(double x, double y, double z) {
        this.x = x;
        this.y = y;
        this.z = z;
    }
}
