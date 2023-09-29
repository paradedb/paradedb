// pages/index.js
'use client';
import { useUser } from '@auth0/nextjs-auth0/client';
import { hostname } from 'os';
import { useState, useEffect } from 'react';
import './page.css';

export default function Index() {
    const { user, error, isLoading } = useUser();
    const [isNewUser, setisNewUser] = useState(false);
    const [creds, setCreds] = useState(null);

    useEffect(() => {
        const fetchUserCreds = async () => {
            try {
                const response = await fetch("/api/cloud", { method: 'GET' });
                if (!response.ok && response.status == 404) {
                    setisNewUser(true);
                } else if (!response.ok) {
                    throw new Error('Server error');
                }

                const data = await response.json();
                setCreds({ host: data["host"], port: data["port"], user: data["user"], password: data["password"] });
            } catch (error) {
                console.error("There was a problem with the fetch operation:", error);
            }
        };

        fetchUserCreds();
    }, []); // Empty dependency array means this useEffect runs once when the component mounts

    const signupFreePlan = async () => {
        try {
            const response = await fetch("/api/cloud", { method: 'POST' });
            if (!response.ok) {
                throw new Error('Server Error');
            }
            setisNewUser(false);
        } catch (error) {
            console.error("There was a problem with the fetch operation:", error);
        }

    }

    if (isLoading) return <div>Loading...</div>;
    if (error) return <div>{error.message}</div>;

    if (user) {
        return (
            <div className={"container"}>
                <div className={"userInfo"}>
                    Welcome {user.name}! <a href="/api/auth/logout">Logout</a>
                </div>
                {
                    isNewUser ?
                        <div className={"buttonContainer"}>
                            <button className={"freePlanButton"} onClick={signupFreePlan}>Sign up for free plan</button>
                        </div> :
                        <div className={"buttonContainer"}>
                            {creds != null ? <p>Your connection string is <br></br>host={creds.host} port={creds.port} user={creds.user} password={creds.password}</p> : <p>User already exists</p>}
                        </div>
                }

            </div>
        );
    }

    return <a href="/api/auth/login">Login</a>;
}